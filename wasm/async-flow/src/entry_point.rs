use std::rc::Rc;

use futures::StreamExt;
use wasm_bindgen::prelude::*;
use wasm_utils::{animation::AnimationLoop, error::*, info};
use web_sys::{Element, HtmlCanvasElement};
use webgl2::{
    context::{Context, COLOR_BLACK},
    font::{Align, TextShader},
};

use crate::ui::{first, request, second};

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
    let (ui3, mut rx3) = crate::ui::request::start()?;

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

    wasm_bindgen_futures::spawn_local(async move {
        let ui = ui3;

        while let Some(event) = rx3.next().await {
            // リクエスト処理中はsubmitボタンを無効化
            ui.enable(false);
            match event {
                request::Event::Submit => {
                    info!("request::Event::Submit");
                    let dur = ui.duration();
                    // ここからリクエストを送信する。
                    // この1フローだけではUIからの入力のキャンセルなどは受け付けられない
                    for _ in 0..ui.times() {
                        let res = gloo_net::http::Request::get(&format!(
                            "http://localhost:8080/api/sleep/{dur}"
                        ))
                        .send()
                        .await
                        .expect("Failed to fetch");
                        let text = res.text().await.expect("Failed to get text");
                        info!("res: {text}");
                    }
                }
                _ => {}
            }
            ui.enable(true);
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

/// エレメント取得のラッパー
fn get_element<T>(id: &str) -> Result<T>
where
    T: wasm_bindgen::JsCast,
{
    web_sys::window()
        .ok_or(JsError::new("Failed to get window"))?
        .document()
        .ok_or(JsError::new("Failed to get document"))?
        .get_element_by_id(id)
        .ok_or(JsError::new(&format!("Failed to get element: {id}")))?
        .dyn_into::<T>()
        .map_err(|_| JsError::new(&format!("Failed to convert Element: {id}")))
}
