use futures::StreamExt;
use wasm_bindgen::prelude::*;
use wasm_utils::{error::*, info};
use web_sys::HtmlCanvasElement;

use crate::input::SubmitBtn;

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

    let (tx, mut rx) = futures::channel::mpsc::channel(1);
    let submit_btn = SubmitBtn::new("submit-btn")?;
    submit_btn.start(tx)?;

    wasm_bindgen_futures::spawn_local(async move {
        info!("spawn_local");
        loop {
            // wait message
            let x = rx.next().await;
            info!("x: {:?}", x);
        }
        info!("exit");
    });

    Ok(())
}
