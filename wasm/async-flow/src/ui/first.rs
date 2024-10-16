//! 1つ目のUI操作グループ

use futures::channel::mpsc::Receiver;
use wasm_bindgen::prelude::*;

use wasm_utils::{
    error::*,
    input::{
        button::{CheckBox, SubmitBtn},
        slider::{SliderConfig, SliderInput},
        InputBool, InputIdent, InputNumber,
    },
};

/// 識別子と値を分けずにメッセージの型を定義する
///
/// 1つのチャネルを通じてUIから値を返してくる型の一覧で
/// メッセージの識別と値のペアで構成される
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Submit,
    Toggle(bool),
    Slider1(f32),
    Slider2(u16),
    Slider3(f32),
}

impl InputIdent for Event {
    fn id(&self) -> &'static str {
        match self {
            Event::Submit => "submit-btn",
            Event::Toggle(_) => "toggle-btn",
            Event::Slider1(_) => "slider1",
            Event::Slider2(_) => "slider2",
            Event::Slider3(_) => "slider3",
        }
    }
}

impl InputBool for Event {
    fn value(&self) -> Result<bool> {
        match self {
            Event::Toggle(b) => Ok(*b),
            _ => Err(JsError::new("not bool")),
        }
    }
    fn with_value(&self, value: bool) -> Result<Self> {
        match self {
            Event::Toggle(_) => Ok(Event::Toggle(value)),
            _ => Err(JsError::new("not bool")),
        }
    }
}

impl InputNumber<f32> for Event {
    fn value(&self) -> Result<f32> {
        match self {
            Event::Slider1(f) => Ok(*f),
            Event::Slider3(f) => Ok(*f),
            _ => Err(JsError::new("not f32")),
        }
    }
    fn with_value(&self, value: f32) -> Result<Self> {
        match self {
            Event::Slider1(_) => Ok(Event::Slider1(value)),
            Event::Slider3(_) => Ok(Event::Slider3(value)),
            _ => Err(JsError::new("not f32")),
        }
    }
}

impl InputNumber<u16> for Event {
    fn value(&self) -> Result<u16> {
        match self {
            Event::Slider2(u) => Ok(*u),
            _ => Err(JsError::new("not u16")),
        }
    }
    fn with_value(&self, value: u16) -> Result<Self> {
        match self {
            Event::Slider2(_) => Ok(Event::Slider2(value)),
            _ => Err(JsError::new("not u16")),
        }
    }
}

/// プログラム側からUIを操作するための構造体
pub struct Ui {
    submit_btn: SubmitBtn<Event>,
    toggle_btn: CheckBox<Event>,
    slider1: SliderInput<Event, f32>,
    slider2: SliderInput<Event, u16>,
    slider3: SliderInput<Event, f32>,
}

impl Ui {
    pub fn new() -> Result<Self> {
        let submit_btn = SubmitBtn::new(Event::Submit)?;
        let toggle_btn = CheckBox::new(Event::Toggle(true))?;
        let slider1 = SliderInput::new(
            Event::Slider1(0.1),
            SliderConfig::new(-1.0, 1.0, 0.1_f32, 0.1),
        )?;
        let slider2 = SliderInput::new(Event::Slider2(20), SliderConfig::new(0, 60, 10, 0))?;
        let slider3 =
            SliderInput::new(Event::Slider3(2.0), SliderConfig::new(-10.0, 3.0, 0.5, 2.0))?;
        Ok(Self {
            submit_btn,
            toggle_btn,
            slider1,
            slider2,
            slider3,
        })
    }

    /// イベントリスナーを登録して入力を受け付ける
    pub fn start(&self, tx: futures::channel::mpsc::Sender<Event>) -> Result<()> {
        self.submit_btn.start(tx.clone())?;
        self.toggle_btn.start(tx.clone())?;
        self.slider1.start(tx.clone())?;
        self.slider2.start(tx.clone())?;
        self.slider3.start(tx)?;
        Ok(())
    }

    /// プログラム側からUIへのイベント適用
    pub fn apply(&self, event: Event) {
        match event {
            Event::Toggle(b) => {
                self.toggle_btn.apply(b);
            }
            Event::Slider1(f) => {
                self.slider1.apply(f);
            }
            Event::Slider2(u) => {
                self.slider2.apply(u);
            }
            Event::Slider3(f) => {
                self.slider3.apply(f);
            }
            _ => unimplemented!(),
        }
    }

    /// イベントリスナーを削除
    pub fn remove(&self) {
        self.submit_btn.remove();
        self.toggle_btn.remove();
        self.slider1.remove();
        self.slider2.remove();
        self.slider3.remove();
    }
}

pub fn start() -> Result<(Ui, Receiver<Event>)> {
    let (tx, rx) = futures::channel::mpsc::channel(10);
    let ui = Ui::new()?;
    ui.start(tx)?;
    Ok((ui, rx))
}
