use futures::StreamExt;
use wasm_bindgen::prelude::*;
use wasm_utils::{error::*, info};
use web_sys::HtmlCanvasElement;

use crate::input::{
    CheckBox, InputEvent, InputEventValue, InputIdent, OptionExample, SelectInput, SliderConfig,
    SliderInput, SubmitBtn,
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
    Select,
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
            UiIdent::Select => "selectbox",
        }
    }
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    canvas.set_width(1024);
    canvas.set_height(768);

    info!("start");

    let (tx, mut rx) = futures::channel::mpsc::channel(10);
    let submit_btn = SubmitBtn::new(UiIdent::Submit)?;
    submit_btn.start(tx.clone())?;

    let toggle_btn = CheckBox::new(UiIdent::Toggle, true)?;
    toggle_btn.start(tx.clone())?;

    let slider = SliderInput::new(UiIdent::Slider, SliderConfig::new(-1.0, 1.0, 0.1_f32, 0.1))?;
    slider.start(tx)?;

    let (tx, mut rx_sel) = futures::channel::mpsc::channel(1);
    let select = SelectInput::new(UiIdent::Select, OptionExample::Normal)?;
    select.start(tx)?;

    wasm_bindgen_futures::spawn_local(async move {
        info!("spawn_local");
        loop {
            // wait message
            let event = rx.next().await.unwrap();
            info!("event: {:?}", event);
        }
        info!("exit");
    });

    // 制御フローを分ける。更新頻度やUIと値の組み合わせによって更新内容やタイミングが異なるため
    // canvas以外については都度ページが変わるたびにDOMを再構成するという可能性もなくはない?
    wasm_bindgen_futures::spawn_local(async move {
        info!("spawn_local2");
        loop {
            // wait message
            let event = rx_sel.next().await.unwrap();
            info!("event: {:?}", event);
        }
        info!("exit");
    });

    Ok(())
}
