use futures::{select, StreamExt};
use wasm_bindgen::prelude::*;
use wasm_utils::{error::*, info};
use web_sys::HtmlCanvasElement;

use crate::input::{CheckBox, SliderConfig, SliderInput, SubmitBtn};

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

    let (tx, mut rx_sbm) = futures::channel::mpsc::channel(1);
    let submit_btn = SubmitBtn::new("submit-btn")?;
    submit_btn.start(tx)?;

    let (tx, mut rx_tgl) = futures::channel::mpsc::channel(1);
    let toggle_btn = CheckBox::new("toggle-btn", true)?;
    toggle_btn.start(tx)?;

    let (tx, mut rx_f64) = futures::channel::mpsc::channel(1);
    let slider = SliderInput::new("slider", SliderConfig::new(-1.0, 1.0, 0.1_f32, 0.1))?;
    slider.start(tx)?;

    wasm_bindgen_futures::spawn_local(async move {
        info!("spawn_local");
        loop {
            // wait message
            select! {
                v = rx_tgl.next() => {
                    info!("toggle {v:?}");
                }
                v = rx_f64.next() => {
                    info!("slider {v:?}");
                }
                _ = rx_sbm.next() => {
                    info!("submit");
                }
            }
        }
        info!("exit");
    });

    Ok(())
}
