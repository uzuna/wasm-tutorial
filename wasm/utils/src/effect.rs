//! HTML表示だけに作用する実装

use std::{
    fmt::{self, Debug, Formatter},
    vec,
};

use futures_channel::mpsc;
use wasm_bindgen::prelude::Closure;
use web_sys::{HtmlButtonElement, HtmlElement};

use crate::{
    error::*,
    info,
    util::{add_event_listener, get_element, get_elements, remove_event_listener},
};

/// タブによる表示の切り替えの実装
pub struct Tab {
    // 対象となるタブのクラス名。デバッグ用
    button_class: String,
    // クラス名に紐付けられた表示切り替え対象のインスタンス
    elements: Vec<TabPage>,
    // イベント受信用のチャネル
    rx: mpsc::Receiver<TabEvent>,
}

impl Tab {
    /// ボタンクラス名をもとに、対応付けられたタブのインスタンスを生成
    ///
    /// 期待される最小限のHTML構造は以下の通り。
    /// 以下の場合にクラス名`tab-btn`を指定すると、タブの表示切り替えが可能。
    ///
    /// 1. button要素には`value`属性に表示対象のidを指定する。
    /// 2. div要素には表示対象のidを指定する。
    ///
    /// ```html
    /// <button class="tab-btn" value="tab1">Tab1</button>
    /// <button class="tab-btn" value="tab2">Tab2</button>
    /// <div id="tab1" style="display: none;">Tab1</div>
    /// <div id="tab2" style="display: none;">Tab2</div>
    /// ```
    pub fn new(button_class: impl Into<String>) -> Result<Self> {
        let button_class = button_class.into();
        let btns = get_elements::<HtmlButtonElement>(button_class.as_str())?;
        let mut eles = vec![];
        let (tx, rx) = mpsc::channel(1);
        for b in btns {
            match TabPage::try_from_element(b, tx.clone()) {
                Ok(page) => eles.push(page),
                Err(e) => info!("{:?}", e),
            }
        }
        Ok(Self {
            button_class,
            elements: eles,
            rx,
        })
    }

    /// タブの表示切り替えタスクの開始
    pub fn start(mut self) -> Result<()> {
        wasm_bindgen_futures::spawn_local(async move {
            use futures_util::stream::StreamExt;
            loop {
                let event = self.rx.next().await.unwrap();
                match event {
                    TabEvent::Show(value) => {
                        for page in self.elements.iter_mut() {
                            page.set_display(page.body.id() == value);
                        }
                    }
                }
            }
        });
        Ok(())
    }
}

impl Debug for Tab {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tab")
            .field("button_class", &self.button_class)
            .field("elements: ", &self.elements.len())
            .finish()
    }
}

// 表示対応付けされたボタンとボディ、ボタンに紐付けられたクロージャ
struct TabPage {
    closure: Closure<dyn FnMut()>,
    button: HtmlButtonElement,
    body: HtmlElement,
}

impl TabPage {
    const EVENT_TYPE: &'static str = "click";
    fn try_from_element(button: HtmlButtonElement, tx: mpsc::Sender<TabEvent>) -> Result<Self> {
        // buttonにvalueがあるか確認し、あるならidでbodyを検索
        let value = button.value();
        let body = get_element::<HtmlElement>(&value)?;
        let closure = Closure::wrap(Box::new(move || {
            tx.clone().try_send(TabEvent::show(&value)).unwrap();
        }) as Box<dyn FnMut()>);
        add_event_listener(&button, Self::EVENT_TYPE, closure.as_ref())?;
        Ok(Self {
            closure,
            button,
            body,
        })
    }

    fn set_display(&self, display: bool) {
        self.body
            .style()
            .set_property("display", if display { "block" } else { "none" })
            .unwrap();
        self.button.set_disabled(display);
    }
}

impl Drop for TabPage {
    fn drop(&mut self) {
        // 生成時にイベント登録しているため、基本的に失敗しない
        remove_event_listener(&self.button, Self::EVENT_TYPE, self.closure.as_ref()).unwrap();
    }
}

#[derive(Debug)]
enum TabEvent {
    /// 指定のタブを表示
    Show(String),
}

impl TabEvent {
    fn show(value: impl Into<String>) -> Self {
        Self::Show(value.into())
    }
}
