use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

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
        let element = get_html_element::<web_sys::HtmlButtonElement>(id)?;
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

/// レベルトリガーのチェックボックス
pub struct CheckBox {
    id: String,
    element: web_sys::HtmlInputElement,
    state: Rc<RefCell<AtomicBool>>,
}

impl CheckBox {
    pub fn new(id: &str, initial_value: bool) -> Result<Self> {
        let element = get_html_element::<web_sys::HtmlInputElement>(id)?;
        let state = Rc::new(RefCell::new(AtomicBool::new(initial_value)));

        let s = Self {
            id: id.to_string(),
            element,
            state,
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
    pub fn start(&self, mut tx: futures::channel::mpsc::Sender<bool>) -> Result<()> {
        // check closure
        if contains(&self.id) {
            return Err(JsError::new("Closure already exists"));
        }
        let ele = self.element.clone();
        let state = self.state.clone();
        let closure = Closure::wrap(Box::new(move || {
            let next = !state.borrow().load(Ordering::Relaxed);
            let state = state.borrow_mut();
            state.store(next, Ordering::Relaxed);
            // send message with sync
            tx.try_send(next).unwrap();
        }) as Box<dyn FnMut()>);
        ele.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        // register closure
        insert(&self.id, closure);
        Ok(())
    }

    /// プログラム側から状態を変更する
    pub fn apply(&self, value: bool) {
        self.state.borrow_mut().store(value, Ordering::Relaxed);
        self.element.set_checked(value);
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

fn get_html_element<T>(id: &str) -> Result<T>
where
    T: wasm_bindgen::JsCast,
{
    web_sys::window()
        .ok_or(JsError::new("Failed to get window"))?
        .document()
        .ok_or(JsError::new("Failed to get document"))?
        .get_element_by_id(id)
        .ok_or(JsError::new("Failed to get element"))?
        .dyn_into::<T>()
        .map_err(|_| JsError::new("Failed to convert to HtmlButtonElement"))
}
