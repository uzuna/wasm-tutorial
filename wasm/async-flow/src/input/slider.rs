use std::{cell::RefCell, fmt::Debug, rc::Rc, str::FromStr};

use futures::channel::mpsc;
use wasm_bindgen::prelude::*;
use wasm_utils::error::*;

use super::{util::*, InputIdent, InputNumber};

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
            tx.try_send(ident.with_value(value).unwrap()).unwrap();
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
}
