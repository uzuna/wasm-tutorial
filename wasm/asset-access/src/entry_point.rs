use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_utils::error::*;
use web_sys::{HtmlCanvasElement, HtmlImageElement};

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
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

    // 画像を非同期で読み込んで描画
    spawn_local(async move {
        let img = HtmlImageElement::new().unwrap();
        let img = Rc::new(img);
        img.set_src("../resources/fonts/Ubuntu_Mono_64px.png");
        let img_clone = img.clone();
        let closure = Closure::wrap(Box::new(move || {
            let texture = webgl2::shader::texture::create_texture_image_element(&gl, &img_clone);
            s.draw(&v, &texture);
        }) as Box<dyn FnMut()>);
        let _ = img.add_event_listener_with_callback("load", closure.as_ref().unchecked_ref());

        // TODO: 大量のデータを読み込むときは、forgetせず、imgもclosureも破棄したいがどうする?
        // https://docs.rs/gloo-events/0.2.0/gloo_events/struct.EventListener.html
        closure.forget();
    });

    Ok(())
}
