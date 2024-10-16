use std::{cell::RefCell, fmt::Debug, rc::Rc, str::FromStr};

use futures_channel::mpsc;
use wasm_bindgen::prelude::*;

use super::{util::*, InputIdent, InputNumber};
use crate::error::*;

/// スライダエレメントの設定を作る
#[derive(Debug, Clone)]
pub struct SliderConfig<T> {
    // 設定範囲とステップ、初期値を設定
    pub min: T,
    pub max: T,
    pub step: T,
    pub default: T,
}

impl<T> SliderConfig<T>
where
    T: ToString,
{
    pub fn new(min: T, max: T, step: T, default: T) -> Self {
        Self {
            min,
            max,
            step,
            default,
        }
    }

    fn apply(&self, ele: &web_sys::HtmlInputElement) {
        ele.set_min(&self.min.to_string());
        ele.set_max(&self.max.to_string());
        ele.set_step(&self.step.to_string());
        ele.set_value(&self.default.to_string());
    }
}

pub trait SliderFormat<T>: Clone {
    fn format(&self, value: &T) -> String;
}

/// スライダーの出力フォーマット
#[derive(Debug, Clone)]
pub struct OutputFmt<T, F>
where
    F: SliderFormat<T>,
{
    // 出力先
    e: web_sys::Element,
    fmt: F,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, F> OutputFmt<T, F>
where
    T: ToString,
    F: SliderFormat<T>,
{
    pub fn new(e: web_sys::Element, fmt: F) -> Self {
        Self {
            e,
            fmt,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn by_id(id: &str, fmt: F) -> Result<Self> {
        let e = get_element::<web_sys::Element>(id)?;
        Ok(Self::new(e, fmt))
    }

    pub fn apply(&self, value: &T) {
        self.e.set_text_content(Some(&self.fmt.format(value)));
    }
}

#[derive(Clone)]
pub struct RawFmt;

impl<T> SliderFormat<T> for RawFmt
where
    T: ToString,
{
    fn format(&self, value: &T) -> String {
        value.to_string()
    }
}

/// スライダーの実装
///
/// 任意の値域を持ちその値を返す
pub struct SliderInput<I, T>
where
    I: InputIdent,
{
    element: web_sys::HtmlInputElement,
    state: Rc<RefCell<T>>,
    ident: I,
}

impl<I, T> SliderInput<I, T>
where
    I: InputIdent + InputNumber<T>,
    T: Copy + FromStr + ToString + 'static,
{
    pub fn new(ident: I, mut config: SliderConfig<T>) -> Result<Self> {
        let id = ident.id();
        let element = get_element::<web_sys::HtmlInputElement>(id)?;
        let default = ident.value()?;
        config.default = default;
        config.apply(&element);
        let state = Rc::new(RefCell::new(config.default));

        let s = Self {
            element,
            state,
            ident,
        };
        s.init();

        Ok(s)
    }

    // 自身の状態とHTML要素の状態を同期する
    pub fn init(&self) {
        let value = self.state.borrow().to_string();
        self.element.set_value(&value);
    }

    /// イベントリスナーを登録する
    pub fn start(&self, mut tx: mpsc::Sender<I>) -> Result<()> {
        // check closure
        if contains(self.ident.id()) {
            return Err(JsError::new("Closure already exists"));
        }
        let ele = self.element.clone();
        let state = self.state.clone();
        let ident = self.ident.to_owned();
        let closure = Closure::wrap(Box::new(move || {
            let value = match ele.value().parse::<T>() {
                Ok(v) => v,
                Err(_) => return,
            };
            *state.borrow_mut() = value;
            // send message with sync
            match tx.try_send(ident.with_value(value).unwrap()) {
                Ok(_) => (),
                Err(e) => {
                    error!("Failed to send message: {:?}", e);
                }
            }
        }) as Box<dyn FnMut()>);
        self.element
            .set_oninput(Some(closure.as_ref().unchecked_ref()));
        insert(self.ident.id(), closure);
        Ok(())
    }

    /// プログラム側から状態を変更する
    pub fn apply(&self, value: T) {
        self.element.set_value(&value.to_string());
        *self.state.borrow_mut() = value;
    }

    pub fn remove(&self) {
        remove_closure(self.ident.id());
    }

    pub fn value(&self) -> T {
        *self.state.borrow()
    }
}

/// スライダーの実装
///
/// 任意の値域を持ちその値を返す
pub struct SliderInputWithOutput<I, T, F>
where
    I: InputIdent,
    F: SliderFormat<T>,
{
    element: web_sys::HtmlInputElement,
    state: Rc<RefCell<T>>,
    ident: I,
    output: OutputFmt<T, F>,
}

impl<I, T, F> SliderInputWithOutput<I, T, F>
where
    I: InputIdent + InputNumber<T>,
    T: Copy + FromStr + ToString + 'static,
    F: SliderFormat<T> + 'static,
{
    pub fn new(ident: I, mut config: SliderConfig<T>, output: OutputFmt<T, F>) -> Result<Self> {
        let id = ident.id();
        let element = get_element::<web_sys::HtmlInputElement>(id)?;
        let default = ident.value()?;
        config.default = default;
        config.apply(&element);
        let state = Rc::new(RefCell::new(config.default));

        let s = Self {
            element,
            state,
            ident,
            output,
        };
        s.init();

        Ok(s)
    }

    // 自身の状態とHTML要素の状態を同期する
    pub fn init(&self) {
        let value = self.state.borrow().to_string();
        self.element.set_value(&value);
        self.output.apply(&self.state.borrow());
    }

    /// イベントリスナーを登録する
    pub fn start(&self, mut tx: mpsc::Sender<I>) -> Result<()> {
        // check closure
        if contains(self.ident.id()) {
            return Err(JsError::new("Closure already exists"));
        }
        let ele = self.element.clone();
        let state = self.state.clone();
        let output = self.output.clone();
        let ident = self.ident.to_owned();
        let closure = Closure::wrap(Box::new(move || {
            let value = match ele.value().parse::<T>() {
                Ok(v) => v,
                Err(_) => return,
            };
            *state.borrow_mut() = value;
            output.apply(&value);
            // send message with sync
            match tx.try_send(ident.with_value(value).unwrap()) {
                Ok(_) => (),
                Err(e) => {
                    error!("Failed to send message: {:?}", e);
                }
            }
        }) as Box<dyn FnMut()>);
        self.element
            .set_oninput(Some(closure.as_ref().unchecked_ref()));
        insert(self.ident.id(), closure);
        Ok(())
    }

    /// プログラム側から状態を変更する
    pub fn apply(&self, value: T) {
        self.element.set_value(&value.to_string());
        *self.state.borrow_mut() = value;
    }

    pub fn remove(&self) {
        remove_closure(self.ident.id());
    }

    pub fn value(&self) -> T {
        *self.state.borrow()
    }
}
