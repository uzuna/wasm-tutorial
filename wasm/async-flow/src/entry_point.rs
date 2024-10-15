use futures::StreamExt;
use wasm_bindgen::prelude::*;
use wasm_utils::{error::*, info};
use web_sys::HtmlCanvasElement;

use crate::input::{
    CheckBox, InputEvent, InputEventValue, InputIdent, SliderConfig, SliderInput, SubmitBtn,
};

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

/// UIの識別子
///
/// 大量のチャンネルを扱いたくなかったので、UIから届く値とその元についてこちらで定義
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiIdent {
    Submit,
    Toggle,
    Slider,
}

impl InputIdent for UiIdent {
    fn build_event(&self, value: InputEventValue) -> InputEvent<Self> {
        InputEvent::new(*self, value)
    }

    fn id(&self) -> &'static str {
        match self {
            UiIdent::Submit => "submit-btn",
            UiIdent::Toggle => "toggle-btn",
            UiIdent::Slider => "slider",
        }
    }
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    canvas.set_width(1024);
    canvas.set_height(768);

    info!("start");

    let (tx, mut rx) = futures::channel::mpsc::channel(1);
    let submit_btn = SubmitBtn::new(UiIdent::Submit)?;
    submit_btn.start(tx.clone())?;

    let toggle_btn = CheckBox::new(UiIdent::Toggle, true)?;
    toggle_btn.start(tx.clone())?;

    let slider = SliderInput::new(UiIdent::Slider, SliderConfig::new(-1.0, 1.0, 0.1_f32, 0.1))?;
    slider.start(tx)?;

    wasm_bindgen_futures::spawn_local(async move {
        info!("spawn_local");
        loop {
            // wait message
            let event = rx.next().await.unwrap();
            info!("event: {:?}", event);
        }
        info!("exit");
    });

    Ok(())
}
