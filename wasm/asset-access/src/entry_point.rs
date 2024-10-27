//! サーバーから画像を読み込んでテクスチャとして使う例

use core::f32;
use std::rc::Rc;

use nalgebra::Vector2;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_utils::{animation::AnimationLoop, error::*, info};
use web_sys::HtmlCanvasElement;
use webgl2::{
    context::{gl_clear_color, COLOR_BLACK},
    gl,
    loader::{load_texture, ImageLoader},
    shader::texture::{TextureShader, TextureVd},
    texture::Texture,
};

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

    let glctx = webgl2::context::Context::new(canvas, webgl2::context::COLOR_BLACK)?;
    let vp = glctx.viewport();

    let mut ctx = DrawContext {
        gl: glctx.gl().clone(),
        objects: vec![],
    };

    let metrics = glctx.metrics().clone();
    let mut textures = vec![];

    // テクスチャインタンスの生成と配置
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
        // 同期処理から非同期にタスクを生成
        // ロードの完了を待たない
        spawn_load_texture(create_img_src(i, color_front.as_str()), texture.clone());
        textures.push(texture.clone());
        ctx.objects.push(Drawable {
            shader: s,
            vao: v,
            texture,
        });
    }

    check_memory_usage("after spawn");

    // console.logにメモリの使用量などを出す
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

    // animation loop
    let mut a = AnimationLoop::new(move |_time| {
        ctx.draw();
        Ok(())
    });
    a.start();
    a.forget();

    // メモリリークの有無を確認するためにテクスチャを定期的に読み出す
    // 実際にforgetではメモリ使用量が増える付けることが確認できた
    spawn_local(async move {
        use futures_util::stream::StreamExt;
        let interval = std::time::Duration::from_secs(5);
        let mut counter = 0;

        // 画像読み出しの同時実行数
        let par_count = 8;
        let requests = textures.iter().enumerate().collect::<Vec<_>>();
        loop {
            let timeout = gloo_timers::future::TimeoutFuture::new(interval.as_millis() as u32);

            // ループごとに画像の色を変える
            let f = match counter % 3 {
                0 => |i| rgba_to_hexcode(i as u8, 0, 128, 255),
                1 => |i| rgba_to_hexcode(128, i as u8, 0, 255),
                _ => |i| rgba_to_hexcode(0, 128, i as u8, 255),
            };

            // 直接に読むと遅いので一定数の画像を同時に読み出す
            futures::stream::iter(requests.iter())
                .for_each_concurrent(par_count, |(i, texture)| async {
                    let color_front = f(*i);
                    let src = create_img_src(*i, color_front.as_str());
                    load_texture(src, texture).await.unwrap();
                })
                .await;
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

// 描画オブジェクトをまとめて保持する構造体
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
