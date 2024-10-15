use std::{
    cell::RefCell,
    rc::Rc,
    str::FromStr,
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
        let closure = Closure::wrap(Box::new(move || {
            // send message with sync
            tx.try_send(()).unwrap();
        }) as Box<dyn FnMut()>);
        add_event_listener(
            self.element.dyn_ref::<web_sys::EventTarget>().unwrap(),
            "click",
            closure.as_ref(),
        )?;
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
        let state = self.state.clone();
        let closure = Closure::wrap(Box::new(move || {
            let next = !state.borrow().load(Ordering::Relaxed);
            let state = state.borrow_mut();
            state.store(next, Ordering::Relaxed);
            // send message with sync
            tx.try_send(next).unwrap();
        }) as Box<dyn FnMut()>);

        self.element
            .set_oninput(Some(closure.as_ref().unchecked_ref()));
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

pub trait SliderValue: Default + Clone + ToString + FromStr + 'static {}

impl SliderValue for u8 {}
impl SliderValue for u16 {}
impl SliderValue for u32 {}
impl SliderValue for u64 {}
impl SliderValue for i8 {}
impl SliderValue for i16 {}
impl SliderValue for i32 {}
impl SliderValue for i64 {}
impl SliderValue for f32 {}
impl SliderValue for f64 {}

/// スライダエレメントの設定を作る
#[derive(Debug, Clone)]
pub struct SliderConfig<T>
where
    T: SliderValue,
{
    // 設定範囲とステップ、初期値を設定
    pub min: T,
    pub max: T,
    pub step: T,
    pub default: T,
}

impl<T> SliderConfig<T>
where
    T: SliderValue,
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

pub struct SliderInput<T>
where
    T: SliderValue,
{
    id: String,
    element: web_sys::HtmlInputElement,
    state: Rc<RefCell<T>>,
}

impl<T> SliderInput<T>
where
    T: SliderValue,
{
    pub fn new(id: &str, config: SliderConfig<T>) -> Result<Self> {
        let element = get_html_element::<web_sys::HtmlInputElement>(id)?;
        config.apply(&element);
        let state = Rc::new(RefCell::new(config.default.clone()));

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
        let value = self.state.borrow().to_string();
        self.element.set_value(&value);
    }

    /// イベントリスナーを登録する
    pub fn start(&self, mut tx: futures::channel::mpsc::Sender<T>) -> Result<()> {
        // check closure
        if contains(&self.id) {
            return Err(JsError::new("Closure already exists"));
        }
        let ele = self.element.clone();
        let state = self.state.clone();
        let closure = Closure::wrap(Box::new(move || {
            let value = match ele.value().parse::<T>() {
                Ok(v) => v,
                Err(_) => return,
            };
            *state.borrow_mut() = value.clone();
            // send message with sync
            tx.try_send(value).unwrap();
        }) as Box<dyn FnMut()>);
        self.element
            .set_oninput(Some(closure.as_ref().unchecked_ref()));
        insert(&self.id, closure);
        Ok(())
    }

    /// プログラム側から状態を変更する
    pub fn apply(&self, value: T) {
        self.element.set_value(&value.to_string());
        *self.state.borrow_mut() = value;
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

/// イベントリスナーを登録する
fn add_event_listener(
    element: &web_sys::EventTarget,
    event: &str,
    callback: &JsValue,
) -> Result<()> {
    element
        .add_event_listener_with_callback(event, callback.unchecked_ref())
        .map_err(|_| JsError::new("Failed to add event listener"))?;
    Ok(())
}
