//! 2つめのUI操作グループ

use futures::channel::mpsc::Receiver;
use wasm_bindgen::prelude::*;
use wasm_utils::error::*;

use crate::input::{select::SelectInput, InputIdent, InputOption, SelectOption};

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
pub enum InputEvent {
    Select1(OptionMode),
    Select2(OptionStrength),
}

impl InputIdent for InputEvent {
    fn id(&self) -> &'static str {
        match self {
            InputEvent::Select1(_) => "selectbox1",
            InputEvent::Select2(_) => "selectbox2",
        }
    }
}

impl InputOption<OptionMode> for InputEvent {
    fn value(&self) -> Result<OptionMode> {
        match self {
            InputEvent::Select1(v) => Ok(*v),
            InputEvent::Select2(_) => Err(JsError::new("not OptionMode")),
            _ => Err(JsError::new("not OptionMode")),
        }
    }
    fn with_value(&self, value: OptionMode) -> Result<Self> {
        match self {
            InputEvent::Select1(_) => Ok(InputEvent::Select1(value)),
            InputEvent::Select2(_) => Err(JsError::new("not OptionMode")),
            _ => Err(JsError::new("not OptionMode")),
        }
    }
}

impl InputOption<OptionStrength> for InputEvent {
    fn value(&self) -> Result<OptionStrength> {
        match self {
            InputEvent::Select1(_) => Err(JsError::new("not OptionStrength")),
            InputEvent::Select2(v) => Ok(*v),
            _ => Err(JsError::new("not OptionStrength")),
        }
    }
    fn with_value(&self, value: OptionStrength) -> Result<Self> {
        match self {
            InputEvent::Select1(_) => Err(JsError::new("not OptionStrength")),
            InputEvent::Select2(_) => Ok(InputEvent::Select2(value)),
            _ => Err(JsError::new("not OptionStrength")),
        }
    }
}

pub fn start() -> Result<Receiver<InputEvent>> {
    let (tx, rx) = futures::channel::mpsc::channel(1);
    let select =
        SelectInput::<InputEvent, OptionMode>::new(InputEvent::Select1(OptionMode::Normal))?;
    select.start(tx.clone())?;

    let select = SelectInput::<InputEvent, OptionStrength>::new(InputEvent::Select2(
        OptionStrength::Strict,
    ))?;
    select.start(tx)?;
    Ok(rx)
}
