use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_utils::{error::*, info};
use web_sys::{HtmlCanvasElement, HtmlImageElement};

thread_local! {
    static LOAD_CLOSUER: RefCell<Option<Closure<dyn FnMut()>>> = RefCell::new(None);
}

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    check_memory_usage("start");
    let width = 500;
    let height = 300;
    canvas.set_width(width);
    canvas.set_height(height);

    let gl = webgl2::context::get_context(&canvas, webgl2::context::COLOR_BLACK)?;
    let gl = Rc::new(gl);

    let s = webgl2::shader::texture::TextureShader::new(gl.clone())?;
    let v = s.create_vao(&webgl2::vertex::UNIT_RECT)?;
    let texture = webgl2::shader::texture::color_texture(&gl, [0, 128, 0, 255]);
    // 事前に緑色のテクスチャを描画
    s.draw(&v, &texture);

    check_memory_usage("before spawn_local");
    // 画像を非同期で読み込んで描画
    spawn_local(async move {
        let img = HtmlImageElement::new().unwrap();
        let img = Rc::new(img);
        img.set_src("../resources/fonts/Ubuntu_Mono_64px.png");
        let img_clone = img.clone();
        let closure = Closure::wrap(Box::new(move || {
            let texture = webgl2::shader::texture::create_texture_image_element(&gl, &img_clone);
            s.draw(&v, &texture);
            // manually drop closure
            LOAD_CLOSUER.with(|c| if let Some(_) = c.borrow_mut().take() {});
            check_memory_usage("take closure");
        }) as Box<dyn FnMut()>);
        check_memory_usage("create closure");
        let _ = img.add_event_listener_with_callback("load", closure.as_ref().unchecked_ref());

        // 大量のデータを読み込むときは、forgetせず、imgもclosureも破棄したいがどうする?
        // https://docs.rs/gloo-events/0.2.0/gloo_events/struct.EventListener.html
        LOAD_CLOSUER.with(|c| {
            *c.borrow_mut() = Some(closure);
        });
    });

    Ok(())
}

// WebAssembly.Memoryの使用量をログ出力
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
