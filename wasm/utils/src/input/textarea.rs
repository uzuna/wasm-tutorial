use futures_channel::mpsc;
use wasm_bindgen::prelude::*;

use super::{util::*, InputIdent, InputString};
use crate::error::*;

/// チェックボックス向けの実装
///
/// boolの状態を持つレベルトリガーのような役割をもつ
pub struct TextArea<I> {
    element: web_sys::HtmlTextAreaElement,
    ident: I,
}

impl<I> TextArea<I>
where
    I: InputIdent + InputString,
{
    pub fn new(ident: I) -> Result<Self> {
        let id = ident.id();
        let element = get_element::<web_sys::HtmlTextAreaElement>(id)?;

        // init
        element.set_value(&ident.value()?);
        Ok(Self { element, ident })
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
        let ele = self.element.clone();
        let closure = Closure::wrap(Box::new(move || {
            let text = ele.value();
            // send message with sync
            tx.try_send(ident.with_value(text).unwrap()).unwrap();
        }) as Box<dyn FnMut()>);

        self.element
            .set_oninput(Some(closure.as_ref().unchecked_ref()));
        // register closure
        insert(self.ident.id(), closure);
        Ok(())
    }

    /// プログラム側から状態を変更する
    pub fn apply(&self, value: &str) {
        self.element.set_value(value);
    }

    pub fn remove(&self) {
        remove_closure(&self.ident.id());
    }
}
