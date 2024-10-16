use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use futures_channel::mpsc;
use wasm_bindgen::prelude::*;

use super::{util::*, InputBool, InputIdent};
use crate::error::*;

/// Submitボタンの実装
///
/// フォーム入力後の送信などエッジトリガーとしての役割を持っている
pub struct SubmitBtn<I> {
    element: web_sys::HtmlButtonElement,
    ident: I,
}

impl<I> SubmitBtn<I>
where
    I: InputIdent,
{
    pub fn new(ident: I) -> Result<Self> {
        let id = ident.id();
        let element = get_element::<web_sys::HtmlButtonElement>(id)?;
        Ok(Self { ident, element })
    }

    pub fn start(&self, mut tx: mpsc::Sender<I>) -> Result<()> {
        // check closure
        if contains(self.ident.id()) {
            return Err(JsError::new(&format!(
                "Closure already exists: {}",
                self.ident.id()
            )));
        }
        let ident = self.ident.clone();
        let closure = Closure::wrap(Box::new(move || {
            // send message with sync
            tx.try_send(ident.clone()).unwrap();
        }) as Box<dyn FnMut()>);
        add_event_listener(
            self.element.dyn_ref::<web_sys::EventTarget>().unwrap(),
            "click",
            closure.as_ref(),
        )?;
        // register closure
        insert(self.ident.id(), closure);
        Ok(())
    }

    pub fn set_text(&self, text: Option<&str>) {
        self.element.set_text_content(text);
    }

    pub fn remove(&self) {
        remove_closure(self.ident.id());
    }

    pub fn enable(&self, enable: bool) {
        self.element.set_disabled(!enable);
    }
}

/// チェックボックス向けの実装
///
/// boolの状態を持つレベルトリガーのような役割をもつ
pub struct CheckBox<I> {
    element: web_sys::HtmlInputElement,
    state: Rc<RefCell<AtomicBool>>,
    ident: I,
}

impl<I> CheckBox<I>
where
    I: InputIdent + InputBool,
{
    pub fn new(ident: I) -> Result<Self> {
        let id = ident.id();
        let element = get_element::<web_sys::HtmlInputElement>(id)?;
        let state = Rc::new(RefCell::new(AtomicBool::new(ident.value()?)));

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
        let next = !self.state.borrow().load(Ordering::Relaxed);
        self.state.borrow_mut().store(next, Ordering::Relaxed);
        self.element.set_checked(next);
    }

    /// イベントリスナーを登録する
    pub fn start(&self, mut tx: mpsc::Sender<I>) -> Result<()> {
        // check closure
        if contains(self.ident.id()) {
            return Err(JsError::new(&format!(
                "Closure already exists: {}",
                self.ident.id()
            )));
        }
        let ident = self.ident.clone();
        let state = self.state.clone();
        let closure = Closure::wrap(Box::new(move || {
            let next = !state.borrow().load(Ordering::Relaxed);
            let state = state.borrow_mut();
            state.store(next, Ordering::Relaxed);
            // send message with sync
            tx.try_send(ident.with_value(next).unwrap()).unwrap();
        }) as Box<dyn FnMut()>);

        self.element
            .set_oninput(Some(closure.as_ref().unchecked_ref()));
        // register closure
        insert(self.ident.id(), closure);
        Ok(())
    }

    /// プログラム側から状態を変更する
    pub fn apply(&self, value: bool) {
        self.state.borrow_mut().store(value, Ordering::Relaxed);
        self.element.set_checked(value);
    }

    pub fn remove(&self) {
        remove_closure(self.ident.id());
    }
}
