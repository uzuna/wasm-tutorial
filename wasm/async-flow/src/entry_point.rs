use futures::StreamExt;
use wasm_bindgen::prelude::*;
use wasm_utils::{error::*, info};
use web_sys::HtmlCanvasElement;

use crate::ui::first::Event;

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    canvas.set_width(1024);
    canvas.set_height(768);

    info!("start");

    let (ui1, mut rx1) = crate::ui::first::start()?;
    let mut rx2 = crate::ui::second::start()?;

    wasm_bindgen_futures::spawn_local(async move {
        info!("spawn_local");
        loop {
            // wait message
            let event = rx1.next().await.unwrap();
            info!("event: {:?}", event);
            match event {
                Event::Submit => {
                    ui1.apply(Event::Slider1(0.1));
                    ui1.apply(Event::Slider2(20));
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
        }
        info!("exit");
    });

    Ok(())
}
