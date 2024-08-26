use core::time;
use std::{cell::RefCell, rc::Rc, sync::atomic::AtomicBool};

use fxhash::FxHashMap;
use gloo_timers::future::sleep;
use tokio::sync::mpsc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_utils::{error::*, info};
use web_sys::{HtmlInputElement, InputEvent};

use crate::storage::{document, local_storage};

thread_local! {
    // テクスチャロードのたびにクロージャをforgetするとメモリリークになるため
    // マニュアルドロップするために一時保存する
    #[allow(clippy::type_complexity)]
    static EVENT_CLOSURES: RefCell<FxHashMap<String,(HtmlInputElement,Closure<dyn FnMut(InputEvent)>)>> = RefCell::new(FxHashMap::default());
}

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start() -> std::result::Result<(), JsValue> {
    let storage = local_storage()?;
    let test_item = storage.get("test")?;
    info!("test_item: {:?}", test_item);
    if test_item.is_none() {
        storage.set("test", "Hello, World!")?;
    }

    let (tx, _rx) = mpsc::unbounded_channel::<C1Prop>();

    // recvは無限ループで処理を専有するので使ってはいけない
    spawn_local(async {
        let mut timer = 0;
        loop {
            sleep(time::Duration::from_millis(20)).await;
            timer += 1;
            info!("timer: {}", timer);
            if timer % 5 == 0 {
                break;
            }
        }
        info!("end timer");
    });

    let mut ctrl = ValueGuiControl::new(C1Prop { scale: 1 }, tx);
    ctrl.attach()?;
    ctrl.start();
    spawn_local(async move {
        sleep(time::Duration::from_secs(5)).await;
        ctrl.detouch();
        info!("end attach");
    });

    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StorageValue {
    pub version: u32,
    pub data: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct C1Prop {
    scale: u32,
}

/// InputElementからのイベントを受け取って、値を更新するデモ
///
/// TODO: スライダの範囲をWASMから設定する
/// 複数の異なる値を設定構造を考える
#[wasm_bindgen]
pub struct ValueGuiControl {
    /// JSのイベントからしかかかれないので、Refcellにしてここに書き込ませる?
    current: Rc<RefCell<C1Prop>>,
    updated: Rc<AtomicBool>,
    _tx: mpsc::UnboundedSender<C1Prop>,
}

impl ValueGuiControl {
    //　TODO: 初期値をstorageから設定し、GUIに反映する
    fn new(current: C1Prop, tx: mpsc::UnboundedSender<C1Prop>) -> Self {
        let updated = Rc::new(AtomicBool::new(false));
        Self {
            current: Rc::new(RefCell::new(current)),
            _tx: tx,
            updated,
        }
    }

    // HTMLへのコールバックの設定
    fn attach(&mut self) -> Result<()> {
        // Callbackは独自の頻度で呼ばれる
        let cache = self.current.clone();
        let flag = self.updated.clone();
        let closure = Closure::wrap(Box::new(move |event: web_sys::InputEvent| {
            let target = event
                .target()
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap();
            info!("input: {:?}", target.value());
            cache.borrow_mut().scale = target.value().parse().unwrap();
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
            // TODO: RcRefCellの中身を変更する
        }) as Box<dyn FnMut(_)>);
        let document = document()?;
        let input = document.get_element_by_id("c1-scale").unwrap();
        let input = input.dyn_into::<web_sys::HtmlInputElement>().unwrap();
        input.set_oninput(Some(closure.as_ref().unchecked_ref()));
        EVENT_CLOSURES.with(|c| {
            c.borrow_mut()
                .insert("c1-scale".to_string(), (input, closure));
        });
        Ok(())
    }

    fn detouch(&mut self) {
        EVENT_CLOSURES.with(|c| {
            let mut c = c.borrow_mut();
            if let Some((e, c)) = c.remove("c1-scale") {
                e.set_oninput(None);
                drop(c);
            }
        });
    }

    // 定期的に更新を確認して、変更があれば送信する処理を入れる
    fn start(&self) {
        let cache = self.current.clone();
        let flag = self.updated.clone();
        spawn_local(async move {
            loop {
                sleep(time::Duration::from_millis(100)).await;
                if flag.load(std::sync::atomic::Ordering::Relaxed) {
                    flag.store(false, std::sync::atomic::Ordering::Relaxed);
                    let prop = cache.borrow().clone();
                    // TODO チャネル送信
                    info!("send: {:?}", prop);
                }
            }
        });
    }
}
