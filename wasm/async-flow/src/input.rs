use std::{
    cell::RefCell,
    fmt::Debug,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use futures::channel::mpsc;
use fxhash::FxHashMap;
use wasm_bindgen::prelude::*;
use wasm_utils::error::*;

thread_local! {
    #[allow(clippy::type_complexity)]
    pub(super) static SELECT_CLOSURES: RefCell<FxHashMap<String,Closure<dyn FnMut()>>> = RefCell::new(FxHashMap::default());
}

/// HTML側のエレメントを探すための識別子を返す
pub trait InputIdent: Copy + 'static {
    fn id(&self) -> &'static str;
}

/// CheckBoxなどのBool型の場合に実装する
pub trait InputBool: Sized {
    fn value(&self) -> Result<bool>;
    fn with_value(&self, value: bool) -> Result<Self>;
}

/// f32型の場合に実装する
pub trait InputF32: Sized {
    fn value(&self) -> Result<f32>;
    fn with_value(&self, value: f32) -> Result<Self>;
}

/// SelectInputの場合に実装する
pub trait InputOption<O>: Sized
where
    O: SelectOption,
{
    fn value(&self) -> Result<O>;
    fn with_value(&self, value: O) -> Result<Self>;
}

/// Submitボタンの実装
///
/// フォーム入力後の送信などエッジトリガーとしての役割を持っている
pub struct SubmitBtn<I> {
    id: String,
    element: web_sys::HtmlButtonElement,
    ident: I,
}

impl<I> SubmitBtn<I>
where
    I: InputIdent,
{
    pub fn new(ident: I) -> Result<Self> {
        let id = ident.id();
        let element = get_html_element::<web_sys::HtmlButtonElement>(id)?;
        Ok(Self {
            id: id.to_string(),
            ident,
            element,
        })
    }

    pub fn start(&self, mut tx: mpsc::Sender<I>) -> Result<()> {
        // check closure
        if contains(&self.id) {
            return Err(JsError::new("Closure already exists"));
        }
        let ident = self.ident;
        let closure = Closure::wrap(Box::new(move || {
            // send message with sync
            tx.try_send(ident).unwrap();
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
        let element = get_html_element::<web_sys::HtmlInputElement>(id)?;
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
        if contains(&self.ident.id()) {
            return Err(JsError::new("Closure already exists"));
        }
        let ident = self.ident;
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
        insert(&self.ident.id(), closure);
        Ok(())
    }

    /// プログラム側から状態を変更する
    pub fn apply(&self, value: bool) {
        self.state.borrow_mut().store(value, Ordering::Relaxed);
        self.element.set_checked(value);
    }
}

/// スライダエレメントの設定を作る
#[derive(Debug, Clone)]
pub struct SliderConfig {
    // 設定範囲とステップ、初期値を設定
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub default: f32,
}

impl SliderConfig {
    pub fn new(min: f32, max: f32, step: f32, default: f32) -> Self {
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
pub struct SliderInput<I>
where
    I: InputIdent,
{
    element: web_sys::HtmlInputElement,
    state: Rc<RefCell<f32>>,
    ident: I,
}

impl<I> SliderInput<I>
where
    I: InputIdent + InputF32,
{
    pub fn new(ident: I, mut config: SliderConfig) -> Result<Self> {
        let id = ident.id();
        let element = get_html_element::<web_sys::HtmlInputElement>(id)?;
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
        if contains(&self.ident.id()) {
            return Err(JsError::new("Closure already exists"));
        }
        let ele = self.element.clone();
        let state = self.state.clone();
        let ident = self.ident.to_owned();
        let closure = Closure::wrap(Box::new(move || {
            let value = match ele.value().parse::<f32>() {
                Ok(v) => v,
                Err(_) => return,
            };
            *state.borrow_mut() = value;
            // send message with sync
            tx.try_send(ident.with_value(value).unwrap()).unwrap();
        }) as Box<dyn FnMut()>);
        self.element
            .set_oninput(Some(closure.as_ref().unchecked_ref()));
        insert(&self.ident.id(), closure);
        Ok(())
    }

    /// プログラム側から状態を変更する
    pub fn apply(&self, value: f32) {
        self.element.set_value(&value.to_string());
        *self.state.borrow_mut() = value;
    }
}

pub trait SelectOption: Copy + Sized + 'static {
    fn iter() -> &'static [Self];
    fn value(&self) -> &str;
    fn text(&self) -> &str;
    fn from_str(value: &str) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionExample {
    Off,
    Normal,
    Dark,
    Bright,
}

impl OptionExample {
    const ALL: [Self; 4] = [Self::Off, Self::Normal, Self::Dark, Self::Bright];
}

impl SelectOption for OptionExample {
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
        let element = get_html_element::<web_sys::HtmlSelectElement>(id)?;
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
            return Err(JsError::new("Closure already exists"));
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
        .ok_or(JsError::new(&format!("Failed to get element: {id}")))?
        .dyn_into::<T>()
        .map_err(|_| JsError::new(&format!("Failed to convert Element: {id}")))
}

fn create_element<T>(tag: &str) -> Result<T>
where
    T: wasm_bindgen::JsCast,
{
    web_sys::window()
        .ok_or(JsError::new("window is None"))?
        .document()
        .ok_or(JsError::new("document is None"))?
        .create_element(tag)
        .map_err(|_| JsError::new("cannot create element"))?
        .dyn_into::<T>()
        .map_err(|_| JsError::new("cannot convert to HtmlElement"))
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
