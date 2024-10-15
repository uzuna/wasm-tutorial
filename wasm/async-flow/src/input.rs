use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use futures::SinkExt;
use fxhash::FxHashMap;
use wasm_bindgen::prelude::*;
use wasm_utils::error::*;

thread_local! {
    #[allow(clippy::type_complexity)]
    pub(super) static SELECT_CLOSURES: RefCell<FxHashMap<String,Closure<dyn FnMut()>>> = RefCell::new(FxHashMap::default());
}

/// AnimationLoopに再生、停止のインタラクションを追加
pub struct SubmitBtn {
    id: String,
    element: web_sys::HtmlButtonElement,
}

impl SubmitBtn {
    pub fn new(id: &str) -> Result<Self> {
        let element = get_html_btn_element(id)?;
        Ok(Self {
            id: id.to_string(),
            element,
        })
    }

    pub fn start(&self, mut tx: futures::channel::mpsc::Sender<()>) -> Result<()> {
        // check closure
        if contains(&self.id) {
            return Err(JsError::new("Closure already exists"));
        }
        let ele = self.element.clone();
        let closure = Closure::wrap(Box::new(move || {
            // send message with sync
            tx.try_send(()).unwrap();
        }) as Box<dyn FnMut()>);
        ele.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        // register closure
        insert(&self.id, closure);
        Ok(())
    }
}

fn contains(id: &str) -> bool {
    SELECT_CLOSURES.with_borrow(|closures| closures.contains_key(id))
}

fn insert(id: &str, closure: Closure<dyn FnMut()>) {
    SELECT_CLOSURES.with(|closures| {
        closures.borrow_mut().insert(id.to_string(), closure);
    });
}

fn get_html_btn_element(id: &str) -> Result<web_sys::HtmlButtonElement> {
    web_sys::window()
        .ok_or(JsError::new("Failed to get window"))?
        .document()
        .ok_or(JsError::new("Failed to get document"))?
        .get_element_by_id(id)
        .ok_or(JsError::new("Failed to get element"))?
        .dyn_into::<web_sys::HtmlButtonElement>()
        .map_err(|_| JsError::new("Failed to convert to HtmlButtonElement"))
}
