//! HTTPリクエストをトリガーするUIの実験

use wasm_bindgen::prelude::*;
use wasm_utils::{
    error::*,
    input::{
        button::SubmitBtn,
        slider::{OutputFmt, SliderConfig, SliderFormat, SliderInputWithOutput},
        InputIdent, InputNumber,
    },
};

/// 識別子と値を分けずにメッセージの型を定義する
///
/// 1つのチャネルを通じてUIから値を返してくる型の一覧で
/// メッセージの識別と値のペアで構成される
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Duration(u32),
    Times(u32),
    Parallel(u32),
    Submit,
}

impl InputIdent for Event {
    fn id(&self) -> &'static str {
        match self {
            Event::Duration(_) => "sleep-duration",
            Event::Times(_) => "sleep-times",
            Event::Parallel(_) => "sleep-parallel",
            Event::Submit => "sleep-apply",
        }
    }
}

impl InputNumber<u32> for Event {
    fn value(&self) -> Result<u32> {
        match self {
            Event::Duration(v) => Ok(*v),
            Event::Times(v) => Ok(*v),
            Event::Parallel(v) => Ok(*v),
            _ => Err(JsError::new("not u32")),
        }
    }
    fn with_value(&self, value: u32) -> Result<Self> {
        match self {
            Event::Duration(_) => Ok(Event::Duration(value)),
            Event::Times(_) => Ok(Event::Times(value)),
            Event::Parallel(_) => Ok(Event::Parallel(value)),
            _ => Err(JsError::new("not u32")),
        }
    }
}

#[derive(Clone)]
pub struct DurationFmt;

impl SliderFormat<u32> for DurationFmt {
    fn format(&self, value: &u32) -> String {
        format!("{} ms", value)
    }
}

#[derive(Clone)]
pub struct TimesFmt;

impl SliderFormat<u32> for TimesFmt {
    fn format(&self, value: &u32) -> String {
        format!("{} times", value)
    }
}

#[derive(Clone)]
pub struct PararellFmt;

impl SliderFormat<u32> for PararellFmt {
    fn format(&self, value: &u32) -> String {
        format!("{} par", value)
    }
}

/// プログラム側からUIを操作するための構造体
pub struct Ui {
    dutation: SliderInputWithOutput<Event, u32, DurationFmt>,
    times: SliderInputWithOutput<Event, u32, TimesFmt>,
    parallel: SliderInputWithOutput<Event, u32, PararellFmt>,
    submit_btn: SubmitBtn<Event>,
}

impl Ui {
    pub fn new() -> Result<Self> {
        let submit_btn = SubmitBtn::new(Event::Submit)?;
        let dur_out = OutputFmt::by_id("sleep-duration-value", DurationFmt)?;
        let dutation = SliderInputWithOutput::new(
            Event::Duration(10),
            SliderConfig::new(10, 1_000, 10, 10),
            dur_out,
        )?;
        let times_out = OutputFmt::by_id("sleep-times-value", TimesFmt)?;
        let times = SliderInputWithOutput::new(
            Event::Times(1),
            SliderConfig::new(1, 100, 1, 1),
            times_out,
        )?;
        let par_out = OutputFmt::by_id("sleep-parallel-value", PararellFmt)?;
        let parallel = SliderInputWithOutput::new(
            Event::Parallel(1),
            SliderConfig::new(1, 10, 1, 1),
            par_out,
        )?;
        Ok(Self {
            dutation,
            times,
            parallel,
            submit_btn,
        })
    }

    pub fn duration(&self) -> u32 {
        self.dutation.value()
    }

    pub fn times(&self) -> u32 {
        self.times.value()
    }

    pub fn parallel(&self) -> u32 {
        self.parallel.value()
    }

    /// イベントリスナーを登録して入力を受け付ける
    pub fn start(&self, tx: futures::channel::mpsc::Sender<Event>) -> Result<()> {
        self.submit_btn.start(tx.clone())?;
        self.dutation.start(tx.clone())?;
        self.times.start(tx.clone())?;
        self.parallel.start(tx)?;
        Ok(())
    }

    /// プログラム側からUIへのイベント適用
    pub fn apply(&self, event: Event) {
        match event {
            Event::Duration(v) => self.dutation.apply(v),
            Event::Times(v) => self.times.apply(v),
            Event::Parallel(v) => self.parallel.apply(v),
            _ => {}
        }
    }

    pub fn enable(&self, enable: bool) {
        self.submit_btn.enable(enable);
    }
}

pub fn start() -> Result<(Ui, futures::channel::mpsc::Receiver<Event>)> {
    let (tx, rx) = futures::channel::mpsc::channel(1);
    let ui = Ui::new()?;
    ui.start(tx)?;
    Ok((ui, rx))
}
