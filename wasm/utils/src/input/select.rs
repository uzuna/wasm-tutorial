use std::{cell::RefCell, rc::Rc};

use futures_channel::mpsc;
use wasm_bindgen::prelude::*;

use super::{util::*, InputIdent, InputOption, SelectOption};
use crate::error::*;

/// セレクトボックスの実装
///
/// 別途指定されたOption型を持ち、その値を返す
pub struct SelectInput<I, O>
where
    I: InputIdent,
    O: SelectOption,
{
    ident: I,
    element: web_sys::HtmlSelectElement,
    state: Rc<RefCell<O>>,
}

impl<I, O> SelectInput<I, O>
where
    I: InputIdent + InputOption<O>,
    O: SelectOption,
{
    pub fn new(ident: I) -> Result<Self> {
        let id = ident.id();
        let element = get_element::<web_sys::HtmlSelectElement>(id)?;
        let state = Rc::new(RefCell::new(ident.value()?));

        let s = Self {
            ident,
            element,
            state,
        };
        s.init()?;

        Ok(s)
    }

    fn init(&self) -> Result<()> {
        for v in O::iter() {
            let option = create_element::<web_sys::HtmlOptionElement>("option")?;
            option.set_value(v.value());
            option.set_text(v.text());
            self.element
                .append_child(option.as_ref())
                .map_err(|e| JsError::new(&format!("failed to append_child {e:?}")))?;
        }
        self.element.set_value(self.state.borrow().value());
        Ok(())
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
        let ele = self.element.clone();
        let state = self.state.clone();
        let ident = self.ident.to_owned();
        let closure = Closure::wrap(Box::new(move || {
            let value = O::from_str(&ele.value());
            *state.borrow_mut() = value;
            // send message with sync
            tx.try_send(ident.with_value(value).unwrap()).unwrap();
        }) as Box<dyn FnMut()>);
        self.element
            .set_oninput(Some(closure.as_ref().unchecked_ref()));
        insert(self.ident.id(), closure);
        Ok(())
    }

    pub fn apply(&self, value: O) {
        *self.state.borrow_mut() = value;
        self.element.set_value(value.value());
    }

    pub fn remove(&self) {
        remove_closure(self.ident.id());
    }
}
