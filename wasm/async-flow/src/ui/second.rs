//! 2つめのUI操作グループ

use futures::channel::mpsc::Receiver;
use wasm_bindgen::prelude::*;
use wasm_utils::{
    error::*,
    input::{select::SelectInput, InputIdent, InputOption, SelectOption},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionMode {
    Off,
    Normal,
    Dark,
    Bright,
}

impl OptionMode {
    const ALL: [Self; 4] = [Self::Off, Self::Normal, Self::Dark, Self::Bright];
}

impl SelectOption for OptionMode {
    fn iter() -> &'static [Self] {
        &Self::ALL
    }

    fn value(&self) -> &str {
        match self {
            Self::Off => "off",
            Self::Normal => "normal",
            Self::Dark => "dark",
            Self::Bright => "bright",
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::Off => "Off",
            Self::Normal => "Normal",
            Self::Dark => "Dark",
            Self::Bright => "Bright",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "off" => Self::Off,
            "normal" => Self::Normal,
            "dark" => Self::Dark,
            "bright" => Self::Bright,
            _ => panic!("invalid value: {}", value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionStrength {
    Off,
    Low,
    High,
    Strict,
}

impl OptionStrength {
    const ALL: [Self; 4] = [Self::Off, Self::Low, Self::High, Self::Strict];
}

impl SelectOption for OptionStrength {
    fn iter() -> &'static [Self] {
        &Self::ALL
    }

    fn value(&self) -> &str {
        match self {
            Self::Off => "off",
            Self::Low => "low",
            Self::High => "high",
            Self::Strict => "strict",
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::Off => "Off",
            Self::Low => "Low",
            Self::High => "High",
            Self::Strict => "Strict",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "off" => Self::Off,
            "low" => Self::Low,
            "high" => Self::High,
            "strict" => Self::Strict,
            _ => panic!("invalid value: {}", value),
        }
    }
}

/// 上3苞は別のチャネルで送受信するメッセージの型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Select1(OptionMode),
    Select2(OptionStrength),
}

impl InputIdent for Event {
    fn id(&self) -> &'static str {
        match self {
            Event::Select1(_) => "selectbox1",
            Event::Select2(_) => "selectbox2",
        }
    }
}

impl InputOption<OptionMode> for Event {
    fn value(&self) -> Result<OptionMode> {
        match self {
            Event::Select1(v) => Ok(*v),
            Event::Select2(_) => Err(JsError::new("not OptionMode")),
        }
    }
    fn with_value(&self, value: OptionMode) -> Result<Self> {
        match self {
            Event::Select1(_) => Ok(Event::Select1(value)),
            Event::Select2(_) => Err(JsError::new("not OptionMode")),
        }
    }
}

impl InputOption<OptionStrength> for Event {
    fn value(&self) -> Result<OptionStrength> {
        match self {
            Event::Select1(_) => Err(JsError::new("not OptionStrength")),
            Event::Select2(v) => Ok(*v),
        }
    }
    fn with_value(&self, value: OptionStrength) -> Result<Self> {
        match self {
            Event::Select1(_) => Err(JsError::new("not OptionStrength")),
            Event::Select2(_) => Ok(Event::Select2(value)),
        }
    }
}

pub struct Ui {
    select1: SelectInput<Event, OptionMode>,
    select2: SelectInput<Event, OptionStrength>,
}

impl Ui {
    pub fn new() -> Result<Self> {
        let select1 = SelectInput::<Event, OptionMode>::new(Event::Select1(OptionMode::Normal))?;
        let select2 =
            SelectInput::<Event, OptionStrength>::new(Event::Select2(OptionStrength::Strict))?;
        Ok(Self { select1, select2 })
    }

    pub fn start(&self, tx: futures::channel::mpsc::Sender<Event>) -> Result<()> {
        self.select1.start(tx.clone())?;
        self.select2.start(tx)?;
        Ok(())
    }

    pub fn apply(&self, event: Event) {
        match event {
            Event::Select1(v) => self.select1.apply(v),
            Event::Select2(v) => self.select2.apply(v),
        }
    }

    pub fn remove(&self) {
        self.select1.remove();
        self.select2.remove();
    }
}

pub fn start() -> Result<(Ui, Receiver<Event>)> {
    let (tx, rx) = futures::channel::mpsc::channel(1);
    let ui = Ui::new()?;
    ui.start(tx)?;
    Ok((ui, rx))
}
