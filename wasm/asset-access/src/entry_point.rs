use core::f32;
use std::{rc::Rc, sync::atomic::AtomicBool};

use futures_util::StreamExt;
use nalgebra::Vector2;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_utils::{
    animation::{
        ctrl::{AnimationCtrl, PlayStopButton},
        AnimationLoop,
    },
    error::*,
    info,
    mouse::{self, MouseEventMessage},
};
use web_sys::HtmlCanvasElement;
use webgl2::{
    context::{gl_clear_color, COLOR_BLACK},
    gl,
    shader::{
        pointing::{PointingRequest, PointingShader},
        texture::{TextureShader, TextureVd},
    },
    texture::Texture,
    GlPoint2d,
};

use crate::loader::ImageLoader;

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    check_memory_usage("start");
    canvas.set_width(1000);
    canvas.set_height(600);
    let canvas_clone = canvas.clone();

    let glctx = webgl2::context::Context::new(canvas, webgl2::context::COLOR_BLACK)?;
    let vp = glctx.viewport();

    let mut ctx = DrawContext {
        gl: glctx.gl().clone(),
        objects: vec![],
    };

    let metrics = glctx.metrics().clone();
    let mut textures = vec![];

    let length = 100;
    for i in 0..length {
        let x = (i as f32 / length as f32 * f32::consts::PI * 2.0).sin();
        let y = (i as f32 / length as f32 * f32::consts::PI * 2.0).cos();
        let s = TextureShader::new(&glctx)?;
        s.uniform().set_mat(
            vp.normalized_unit_mat()
                .append_scaling(0.1)
                .append_translation(&Vector2::new(x / vp.aspect(), y)),
        );
        let v = s.create_vao(&webgl2::vertex::UNIT_RECT)?;
        let texture = glctx.create_blank_texture()?;

        let color_front = rgba_to_hexcode(i as u8, 0, 0, 255);
        spawn_load_texture(create_img_src(i, color_front.as_str()), texture.clone());
        textures.push(texture.clone());
        ctx.objects.push(Drawable {
            shader: s,
            vao: v,
            texture,
        });
    }

    check_memory_usage("after spawn");

    // monitorring resource
    spawn_local(async move {
        use futures_util::{future::ready, stream::StreamExt};
        let interval = std::time::Duration::from_secs(5);
        gloo_timers::future::IntervalStream::new(interval.as_millis() as u32)
            .for_each(|_| {
                info!("closure_length {}", metrics);
                check_memory_usage("monitoring");
                ready(())
            })
            .await;
    });

    // マウスイベントを受け取る
    let (tx, mut rx) = futures_channel::mpsc::unbounded();
    let mut m = mouse::MouseEventHandler::new(canvas_clone, tx);
    m.start();
    m.forget();

    // マウスイベントを元にターゲットを描画するシェーダー
    let mut ts = PointingShader::new(&glctx)?;
    ts.apply_requests(&[
        PointingRequest::Enable(true),
        PointingRequest::Position(GlPoint2d::new(0.0, 0.0)),
    ]);
    ts.update(0.0);
    ts.draw();

    let mut reqs = vec![];
    let mut timestamp = 0.0_f64;
    let a = AnimationLoop::new(move |time| {
        // マウスイベントをターゲットリクエストに変換
        reqs.clear();
        while let Ok(Some(msg)) = rx.try_next() {
            match msg {
                MouseEventMessage::Down { pos } => {
                    reqs.push(PointingRequest::Enable(true));
                    reqs.push(PointingRequest::Position(GlPoint2d::new(pos.x, pos.y)));
                }
                MouseEventMessage::Up { pos } => {
                    reqs.push(PointingRequest::Enable(false));
                    reqs.push(PointingRequest::Position(GlPoint2d::new(pos.x, pos.y)));
                }
                MouseEventMessage::Move { pos } => {
                    reqs.push(PointingRequest::Position(GlPoint2d::new(pos.x, pos.y)));
                }
                MouseEventMessage::Click { pos } => {
                    info!("click {:?}", pos);
                }
                MouseEventMessage::DblClick { pos } => {
                    info!("dblclick {:?}", pos);
                }
                _ => {}
            }
        }
        let elaplesd_sec = (time - timestamp) as f32 / 1000.0;
        timestamp = time;
        ts.apply_requests(reqs.as_slice());
        ts.update(elaplesd_sec);
        // clearが入っているので先に描画
        ctx.draw();
        ts.draw();
        Ok(())
    });

    // ボタンを押すとアニメーションが開始する
    let (tx, mut rx) = futures_channel::mpsc::channel(1);
    let btn = PlayStopButton::new(a, false)?;
    btn.start(tx)?;
    let playing_flag = Rc::new(AtomicBool::new(true));
    let playing_flag_clone = playing_flag.clone();
    wasm_bindgen_futures::spawn_local(async move {
        while let Some(AnimationCtrl::Playing(playing)) = rx.next().await {
            playing_flag_clone.store(playing, std::sync::atomic::Ordering::Relaxed);
        }
    });

    // メモリリークの有無を確認するためにテクスチャを定期的に読み出す
    // 実際にforgetではメモリ使用量が増える付けることが確認できた
    spawn_local(async move {
        use futures_util::stream::StreamExt;
        let interval = std::time::Duration::from_secs(5);
        let mut counter = 0;

        let par_count = 4;
        let requests = textures.iter().enumerate().collect::<Vec<_>>();
        loop {
            info!("load texture {counter}");
            let timeout = gloo_timers::future::TimeoutFuture::new(interval.as_millis() as u32);
            // アニメーションが停止している場合は読み込まない
            if !playing_flag.load(std::sync::atomic::Ordering::Relaxed) {
                info!("playing_flag false");
                timeout.await;
                continue;
            }
            let f = match counter % 3 {
                0 => |i| rgba_to_hexcode(i as u8, 0, 128, 255),
                1 => |i| rgba_to_hexcode(128, i as u8, 0, 255),
                _ => |i| rgba_to_hexcode(0, 128, i as u8, 255),
            };

            info!("start load texture {counter}");
            futures::stream::iter(requests.iter())
                .for_each_concurrent(par_count, |(i, texture)| async {
                    let color_front = f(*i);
                    let src = create_img_src(*i, color_front.as_str());
                    load_texture(src, texture).await.unwrap();
                })
                .await;
            info!("wait timeout {counter}");
            timeout.await;
            counter += 1;
        }
    });

    Ok(())
}

struct Drawable {
    shader: TextureShader,
    vao: webgl2::vertex::Vao<TextureVd>,
    texture: Texture,
}

struct DrawContext {
    gl: Rc<gl>,
    objects: Vec<Drawable>,
}

impl DrawContext {
    fn draw(&self) {
        gl_clear_color(&self.gl, COLOR_BLACK);
        for obj in self.objects.iter() {
            obj.shader.draw(&obj.vao, obj.texture.texture());
        }
    }
}

// テクスチャを先に確保しておき、後から画像を読み込む
fn spawn_load_texture(src: impl AsRef<str>, texture: Texture) {
    let loader = ImageLoader::new(src).unwrap();
    spawn_local(async move {
        let img = loader.await.unwrap();
        texture.update_texture_image_element(&img);
    });
}

async fn load_texture(src: impl AsRef<str>, texture: &Texture) -> Result<()> {
    let loader = ImageLoader::new(src)?;
    let img = loader.await?;
    texture.update_texture_image_element(&img);
    Ok(())
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

fn create_img_src(i: usize, color_front: impl AsRef<str>) -> String {
    format!(
        "../api/texture/generate/test{}?color_front={}",
        i,
        color_front.as_ref()
    )
}
