use futures::StreamExt;
use wasm_bindgen::prelude::*;
use wasm_utils::{error::*, info};
use web_sys::HtmlCanvasElement;

use crate::input::{
    CheckBox, InputBool, InputF32, InputIdent, InputOption, OptionExample, SelectInput,
    SliderConfig, SliderInput, SubmitBtn,
};

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

/// 識別子と値を分けずにメッセージの型を定義する
///
/// 1つのチャネルを通じてUIから値を返してくる型の一覧で
/// メッセージの識別と値のペアで構成される
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PositionMsg {
    Submit,
    Toggle(bool),
    Slider(f32),
}

impl InputIdent for PositionMsg {
    fn id(&self) -> &'static str {
        match self {
            PositionMsg::Submit => "submit-btn",
            PositionMsg::Toggle(_) => "toggle-btn",
            PositionMsg::Slider(_) => "slider",
        }
    }
}

impl InputBool for PositionMsg {
    fn value(&self) -> Result<bool> {
        match self {
            PositionMsg::Toggle(b) => Ok(*b),
            _ => Err(JsError::new("not bool")),
        }
    }
    fn with_value(&self, value: bool) -> Result<Self> {
        match self {
            PositionMsg::Toggle(_) => Ok(PositionMsg::Toggle(value)),
            _ => Err(JsError::new("not bool")),
        }
    }
}

impl InputF32 for PositionMsg {
    fn value(&self) -> Result<f32> {
        match self {
            PositionMsg::Slider(f) => Ok(*f),
            _ => Err(JsError::new("not f32")),
        }
    }
    fn with_value(&self, value: f32) -> Result<Self> {
        match self {
            PositionMsg::Slider(_) => Ok(PositionMsg::Slider(value)),
            _ => Err(JsError::new("not f32")),
        }
    }
}

/// 上3苞は別のチャネルで送受信するメッセージの型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectMsg {
    Select(OptionExample),
}

impl InputIdent for SelectMsg {
    fn id(&self) -> &'static str {
        match self {
            SelectMsg::Select(_) => "selectbox",
        }
    }
}

impl InputOption<OptionExample> for SelectMsg {
    fn value(&self) -> Result<OptionExample> {
        match self {
            SelectMsg::Select(v) => Ok(*v),
        }
    }
    fn with_value(&self, value: OptionExample) -> Result<Self> {
        match self {
            SelectMsg::Select(_) => Ok(SelectMsg::Select(value)),
        }
    }
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> std::result::Result<(), JsValue> {
    canvas.set_width(1024);
    canvas.set_height(768);

    info!("start");

    let (tx, mut rx) = futures::channel::mpsc::channel(10);
    let submit_btn = SubmitBtn::new(PositionMsg::Submit)?;
    submit_btn.start(tx.clone())?;

    let toggle_btn = CheckBox::new(PositionMsg::Toggle(true))?;
    toggle_btn.start(tx.clone())?;

    let slider = SliderInput::new(
        PositionMsg::Slider(0.1),
        SliderConfig::new(-1.0, 1.0, 0.1_f32, 0.1),
    )?;
    slider.start(tx)?;

    let (tx, mut rx_sel) = futures::channel::mpsc::channel(1);
    let select = SelectInput::new(SelectMsg::Select(OptionExample::Normal))?;
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
