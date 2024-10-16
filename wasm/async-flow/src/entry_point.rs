use std::rc::Rc;

use futures::StreamExt;
use wasm_bindgen::prelude::*;
use wasm_utils::{animation::AnimationLoop, error::*, info};
use web_sys::HtmlCanvasElement;
use webgl2::{
    context::{Context, COLOR_BLACK},
    font::{Align, TextShader},
};

use crate::ui::{first, second};

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    canvas.set_width(512);
    canvas.set_height(384);

    info!("start");

    // テキスト表示
    let ctx = Context::new(canvas, COLOR_BLACK)?;
    let gl = ctx.gl().clone();
    let viewport = ctx.viewport();
    let ts = TextShader::new(&ctx)?;
    let font = webgl2::font::embed::load(&ctx)?;
    let mut text = font.text_by_capacity(60, Align::left_bottom());
    let mat = viewport.font_mat(0, 128, 16.0);
    ts.local_mat(&mat);
    let tv = Rc::new(ts.create_vbo(&text)?);
    let tv_clone = tv.clone();

    let (ui1, mut rx1) = crate::ui::first::start()?;
    let (ui2, mut rx2) = crate::ui::second::start()?;

    wasm_bindgen_futures::spawn_local(async move {
        info!("spawn_local");
        loop {
            // wait message
            let event = rx1.next().await.unwrap();
            info!("event: {:?}", event);
            match event {
                first::Event::Submit => {
                    ui1.apply(first::Event::Slider1(0.1));
                    ui1.apply(first::Event::Slider2(20));
                }
                _ => {}
            }
        }
        info!("exit");
    });

    // 制御フローを分ける。更新頻度やUIと値の組み合わせによって更新内容やタイミングが異なるため
    // canvas以外については都度ページが変わるたびにDOMを再構成するという可能性もなくはない?
    wasm_bindgen_futures::spawn_local(async move {
        info!("spawn_local2");
        loop {
            // wait message
            let event = rx2.next().await.unwrap();
            info!("event: {:?}", event);
            match event {
                second::Event::Select1(second::OptionMode::Off) => {
                    ui2.apply(second::Event::Select2(second::OptionStrength::Off));
                }
                second::Event::Text(t) => {
                    text.update_text(&t);
                    text.apply_to_vao(tv_clone.as_ref());
                }
                _ => {}
            }
        }
        info!("exit");
    });

    // Canvasの描画
    let mut a = AnimationLoop::new(move |_time_msec| {
        webgl2::context::gl_clear_color(&gl, webgl2::context::COLOR_BLACK);
        ts.draw(&tv);
        Ok(())
    });
    a.start();
    a.forget();

    info!("start() done");

    Ok(())
}
