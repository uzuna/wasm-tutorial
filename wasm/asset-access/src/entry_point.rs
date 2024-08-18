use core::f32;
use std::{cell::RefCell, rc::Rc};

use fxhash::FxHashMap;
use nalgebra::Vector2;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_utils::{
    animation::{AnimationLoop, PlayStopButton},
    error::*,
    info,
    waitgroup::{WaitGroup, Worker},
};
use web_sys::{HtmlButtonElement, HtmlCanvasElement, HtmlImageElement, WebGlTexture};
use webgl2::{
    context::{gl_clear_color, COLOR_BLACK},
    gl,
    shader::texture::{TextureShader, TextureVd},
};

thread_local! {
    // テクs茶ロードのたびにクロージャをforgetするとメモリリークになるため
    // マニュアルドロップするために一時保存する
    static LOAD_CLOSUER: RefCell<FxHashMap<String,Closure<dyn FnMut()>>> = RefCell::new(FxHashMap::default());
}

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(
    canvas: HtmlCanvasElement,
    play_pause_btn: HtmlButtonElement,
) -> std::result::Result<(), JsValue> {
    check_memory_usage("start");
    let width = 1000;
    let height = 600;
    canvas.set_width(width);
    canvas.set_height(height);
    let vp = webgl2::viewport::ViewPort::new(0, 0, width, height);

    let gl = webgl2::context::get_context(&canvas, webgl2::context::COLOR_BLACK)?;
    let gl = Rc::new(gl);

    let mut ctx = DrawContext {
        gl: gl.clone(),
        objects: vec![],
    };

    let mut textures = vec![];

    let length = 100;
    for i in 0..length {
        let x = (i as f32 / length as f32 * f32::consts::PI * 2.0).sin();
        let y = (i as f32 / length as f32 * f32::consts::PI * 2.0).cos();
        let s = TextureShader::new(gl.clone())?;
        s.uniform().set_mat(
            vp.normalized_unit_mat()
                .append_scaling(0.1)
                .append_translation(&Vector2::new(x / vp.aspect(), y)),
        );
        let v = s.create_vao(&webgl2::vertex::UNIT_RECT)?;
        let texture = webgl2::shader::texture::crate_blank_texture(&gl);
        let texture = Rc::new(texture);

        let color_front = rgba_to_hexcode(i as u8, 0, 0, 255);
        lazy_load_texture(
            gl.clone(),
            format!(
                "../api/texture/generate/test{}?color_front={}",
                i, color_front
            )
            .as_str(),
            texture.clone(),
            None,
        );
        textures.push(texture.clone());
        ctx.objects.push(Drawable {
            shader: s,
            vao: v,
            texture,
        });
    }
    let a = AnimationLoop::new(move |_| {
        ctx.draw();
        Ok(())
    });

    // ボタンを押すとアニメーションが開始する
    let btn = PlayStopButton::new(play_pause_btn, a);
    let playing_flag = btn.flag();
    let ctx = btn.start();
    // JSに戻したらGCで回収されたためforgetする
    ctx.forget();

    check_memory_usage("after spawn");

    // monitorring closure length
    spawn_local(async move {
        use futures_util::{future::ready, stream::StreamExt};
        let interval = std::time::Duration::from_secs(60);
        gloo_timers::future::IntervalStream::new(interval.as_millis() as u32)
            .for_each(|_| {
                let len = LOAD_CLOSUER.with_borrow(|x| x.len());
                info!("closure_length {}", len);
                check_memory_usage("monitoring");
                ready(())
            })
            .await;
    });

    // メモリリークの有無を確認するためにテクスチャを定期的に読み出す
    // 実際にforgetではメモリ使用量が増える付けることが確認できた
    spawn_local(async move {
        use futures_util::{future::ready, stream::StreamExt};
        let interval = std::time::Duration::from_secs(5);
        let mut counter = 0;
        gloo_timers::future::IntervalStream::new(interval.as_millis() as u32)
            .for_each(|_| {
                // アニメーションが停止している場合は読み込まない
                if !playing_flag
                    .borrow()
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    return ready(());
                }
                let wg = WaitGroup::new();
                let f = match counter % 3 {
                    0 => |i| rgba_to_hexcode(i as u8, 0, 128, 255),
                    1 => |i| rgba_to_hexcode(128, i as u8, 0, 255),
                    _ => |i| rgba_to_hexcode(0, 128, i as u8, 255),
                };
                for (i, texture) in textures.iter().enumerate() {
                    let color_front = f(i);
                    lazy_load_texture(
                        gl.clone(),
                        format!(
                            "../api/texture/generate/test{}?color_front={}",
                            i, color_front
                        )
                        .as_str(),
                        texture.clone(),
                        Some(wg.add()),
                    );
                }
                counter += 1;
                spawn_local(async move {
                    wg.wait().await;
                    info!("end load texture");
                });
                ready(())
            })
            .await;
    });

    Ok(())
}

struct Drawable {
    shader: TextureShader,
    vao: webgl2::vertex::Vao<TextureVd>,
    texture: Rc<WebGlTexture>,
}

struct DrawContext {
    gl: Rc<gl>,
    objects: Vec<Drawable>,
}

impl DrawContext {
    fn draw(&self) {
        gl_clear_color(&self.gl, COLOR_BLACK);
        for obj in self.objects.iter() {
            obj.shader.draw(&obj.vao, &obj.texture);
        }
    }
}

// テクスチャを読み込む
#[allow(dead_code)]
fn load_texture(gl: Rc<gl>, src: &str, mut cb: impl FnMut(web_sys::WebGlTexture) + 'static) {
    let src = src.to_string();
    spawn_local(async move {
        let img = HtmlImageElement::new().unwrap();
        let img = Rc::new(img);
        img.set_src(&src);
        let img_clone = img.clone();
        let del_key = src.to_string();
        let closure = Closure::wrap(Box::new(move || {
            let texture = webgl2::shader::texture::create_texture_image_element(&gl, &img_clone);
            // manually drop closure
            LOAD_CLOSUER.with(|c| {
                c.borrow_mut().remove(&del_key);
            });
            img_clone.remove();
            cb(texture);
        }) as Box<dyn FnMut()>);
        let _ = img.add_event_listener_with_callback("load", closure.as_ref().unchecked_ref());
        LOAD_CLOSUER.with(|c| {
            c.borrow_mut().insert(src, closure);
        });
    });
}

// テクスチャを先に確保しておき、後から画像を読み込む
fn lazy_load_texture(gl: Rc<gl>, src: &str, texture: Rc<WebGlTexture>, mut worker: Option<Worker>) {
    let src = src.to_string();
    spawn_local(async move {
        let img = HtmlImageElement::new().unwrap();
        let img = Rc::new(img);
        img.set_src(&src);
        let img_clone = img.clone();
        let del_key = src.to_string();
        let closure = Closure::wrap(Box::new(move || {
            webgl2::shader::texture::rebind_texture(&gl, &texture, &img_clone);
            // manually drop closure
            LOAD_CLOSUER.with(|c| {
                c.borrow_mut().remove(&del_key);
            });
            img_clone.remove();
            if let Some(w) = worker.take() {
                drop(w);
            }
        }) as Box<dyn FnMut()>);
        let _ = img.add_event_listener_with_callback("load", closure.as_ref().unchecked_ref());
        LOAD_CLOSUER.with(|c| {
            c.borrow_mut().insert(src, closure);
        });
    });
}

// WebAssembly.Memoryの使用量をログ出力
// 線形メモリの状態で、growした結果がいつ開放されるのかはよくわからない
// https://developer.mozilla.org/en-US/docs/WebAssembly/JavaScript_interface/Memory
fn check_memory_usage(place: &str) {
    let m = wasm_bindgen::memory()
        .dyn_into::<web_sys::js_sys::WebAssembly::Memory>()
        .expect("should have `memory` on `window`");
    let a = m
        .buffer()
        .dyn_into::<web_sys::js_sys::ArrayBuffer>()
        .expect("should have buffer");
    info!("memory usage [{place}] {} byte", a.byte_length());
}

fn rgba_to_hexcode(r: u8, g: u8, b: u8, a: u8) -> String {
    // queryに含めるために`#`は`%23`にする
    format!("%23{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
}
