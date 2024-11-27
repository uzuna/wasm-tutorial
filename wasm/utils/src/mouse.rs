//! マウスイベントを処理してWASM空間で扱いやすい方にする。

use std::{cell::RefCell, rc::Rc};

use crate::{error::Result, util::get_window};
use futures_channel::mpsc::{UnboundedReceiver, UnboundedSender};
use fxhash::FxHashMap;
use wasm_bindgen::prelude::*;
use web_sys::{AddEventListenerOptions, MouseEvent, WheelEvent};

/// マウス座標を保持、計算する構造体
#[derive(Debug, Default, Clone, Copy)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl std::ops::Sub for Point {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Div for Point {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl std::ops::Div<f32> for Point {
    type Output = Self;
    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl std::ops::Mul<f32> for Point {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

/// ホイールの移動量を保持する構造体
#[derive(Debug, Default, Clone, Copy)]
pub struct Wheel {
    pub x: f32,
    pub y: f32,
}

/// モジュール外にマウスとホイールのイベントを通知する
#[derive(Debug, Clone, Copy)]
pub enum MouseEventMessage {
    Move { pos: Point },
    Wheel { wheel: Wheel },
    Down { pos: Point },
    Up { pos: Point },
    Click { pos: Point },
    DblClick { pos: Point },
    Resize,
}

#[derive(Debug, Clone, Default)]
struct PosCnv {
    inner: Rc<RefCell<PositionConverterInner>>,
}

impl PosCnv {
    fn from_canvas(canvas: &web_sys::HtmlCanvasElement) -> Self {
        Self {
            inner: Rc::new(RefCell::new(PositionConverterInner::from_canvas(canvas))),
        }
    }

    fn update_by_canvas(&self, canvas: &web_sys::HtmlCanvasElement) {
        self.inner.borrow_mut().update_by_canvas(canvas);
    }

    fn pixel_to_gl(&self, p: Point) -> Point {
        self.inner.borrow().pixel_to_gl(p)
    }
}

// Canvasで得られるマウスのpx座標をOpenGLの座標に変換する
#[derive(Debug, Default)]
struct PositionConverterInner {
    offset_c: Point,
    area_c: Point,
}

impl PositionConverterInner {
    fn new(offset: Point, area: Point) -> Self {
        Self {
            offset_c: offset,
            area_c: area,
        }
    }

    fn from_canvas(canvas: &web_sys::HtmlCanvasElement) -> Self {
        let offset = Point {
            x: canvas.offset_left() as f32,
            y: canvas.offset_top() as f32,
        };
        let area = Point {
            x: canvas.width() as f32,
            y: canvas.height() as f32,
        };
        Self::new(offset, area)
    }

    fn update_by_canvas(&mut self, canvas: &web_sys::HtmlCanvasElement) {
        self.offset_c = Point {
            x: canvas.offset_left() as f32,
            y: canvas.offset_top() as f32,
        };
        self.area_c = Point {
            x: canvas.width() as f32,
            y: canvas.height() as f32,
        };
    }

    fn pixel_to_gl(&self, p: Point) -> Point {
        // マウス座標をOpenGL空間に変換
        let mut gl_p = (p - self.offset_c - self.area_c / 2.) / self.area_c * 2.;
        gl_p.y = -gl_p.y;
        gl_p
    }
}

/// マウスイベントを処理する構造体
pub struct MouseEventHandler {
    canvas: web_sys::HtmlCanvasElement,
    cnv: PosCnv,
    mouse_closures: FxHashMap<String, Closure<dyn FnMut(MouseEvent)>>,
    wheel_closures: FxHashMap<String, Closure<dyn FnMut(WheelEvent)>>,
    tx: UnboundedSender<MouseEventMessage>,
    rx: UnboundedReceiver<MouseEventMessage>,
}

impl MouseEventHandler {
    pub fn new(canvas: web_sys::HtmlCanvasElement) -> Self {
        let cnv = PosCnv::from_canvas(&canvas);
        let (tx, rx) = futures_channel::mpsc::unbounded();

        Self {
            canvas,
            cnv,
            mouse_closures: FxHashMap::default(),
            wheel_closures: FxHashMap::default(),
            tx,
            rx,
        }
    }

    pub fn start(&mut self) {
        // マウスの上げ下げイベントは位置と状態を更新
        self.build_mouse_closure("mousedown", |(cnv, event)| {
            let pos = Point::new(event.page_x() as f32, event.page_y() as f32);
            let pos = cnv.pixel_to_gl(pos);
            Some(MouseEventMessage::Down { pos })
        });

        self.build_mouse_closure("mouseup", |(cnv, event)| {
            let pos = Point::new(event.page_x() as f32, event.page_y() as f32);
            let pos = cnv.pixel_to_gl(pos);
            Some(MouseEventMessage::Up { pos })
        });

        // マウス移動は移動のみを取得
        self.build_mouse_closure("mousemove", |(cnv, event)| {
            let pos = Point::new(event.page_x() as f32, event.page_y() as f32);
            let pos = cnv.pixel_to_gl(pos);
            Some(MouseEventMessage::Move { pos })
        });

        self.build_mouse_closure("click", |(cnv, event)| {
            let pos = Point::new(event.page_x() as f32, event.page_y() as f32);
            let pos = cnv.pixel_to_gl(pos);
            Some(MouseEventMessage::Click { pos })
        });

        self.build_mouse_closure("dblclick", |(cnv, event)| {
            let pos = Point::new(event.page_x() as f32, event.page_y() as f32);
            let pos = cnv.pixel_to_gl(pos);
            Some(MouseEventMessage::DblClick { pos })
        });

        // ホイールイベントはホイールの移動量を取得
        self.build_wheel_closure("wheel", |event| {
            Some(MouseEventMessage::Wheel {
                wheel: Wheel {
                    x: event.delta_x() as f32,
                    y: event.delta_y() as f32,
                },
            })
        });

        // リサイズイベントを取得
        self.build_resize_closure();
    }

    fn update_resize(&mut self) {
        self.cnv.update_by_canvas(&self.canvas);
    }

    // マウスイベントのクロージャを登録
    fn build_mouse_closure(
        &mut self,
        event_type: &str,
        f: impl Fn((&PosCnv, MouseEvent)) -> Option<MouseEventMessage> + 'static,
    ) {
        let mut tx = self.tx.clone();
        let cnv = self.cnv.clone();
        let clusure = Closure::wrap(Box::new(move |event: MouseEvent| {
            if let Some(msg) = f((&cnv, event)) {
                tx.start_send(msg).unwrap();
            }
        }) as Box<dyn FnMut(MouseEvent)>);

        self.canvas
            .add_event_listener_with_callback(event_type, clusure.as_ref().unchecked_ref())
            .unwrap();
        self.mouse_closures.insert(event_type.to_string(), clusure);
    }

    // ホイールイベントのクロージャを登録
    fn build_wheel_closure(
        &mut self,
        event_type: &str,
        f: impl Fn(WheelEvent) -> Option<MouseEventMessage> + 'static,
    ) {
        let mut tx = self.tx.clone();
        let clusure = Closure::wrap(Box::new(move |event: WheelEvent| {
            if let Some(msg) = f(event) {
                tx.start_send(msg).unwrap();
            }
        }) as Box<dyn FnMut(WheelEvent)>);

        // スクロール操作というデフォルトのイベントがあるため
        // passive: trueでスクロールイベントをキャンセルしない
        let options = AddEventListenerOptions::new();
        options.set_passive(true);
        self.canvas
            .add_event_listener_with_callback_and_add_event_listener_options(
                event_type,
                clusure.as_ref().unchecked_ref(),
                &options,
            )
            .unwrap();
        self.wheel_closures.insert(event_type.to_string(), clusure);
    }

    fn build_resize_closure(&mut self) {
        let mut tx = self.tx.clone();
        let closure =
            Closure::wrap(
                Box::new(move || tx.start_send(MouseEventMessage::Resize).unwrap())
                    as Box<dyn FnMut()>,
            );

        let vv = get_window().unwrap().visual_viewport().unwrap();
        vv.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        for (event_type, closure) in self.mouse_closures.drain() {
            self.canvas
                .remove_event_listener_with_callback(
                    event_type.as_str(),
                    closure.as_ref().unchecked_ref(),
                )
                .unwrap();
        }
    }

    pub fn try_recv(&mut self) -> Result<Option<MouseEventMessage>> {
        match self.rx.try_next()? {
            Some(msg) => Ok(self.msg_handle(Some(msg))),
            None => Ok(None),
        }
    }

    /// マウスイベントを受信する
    pub async fn recv(&mut self) -> Option<MouseEventMessage> {
        use futures_util::StreamExt;
        let msg = self.rx.next().await;
        self.msg_handle(msg)
    }

    fn msg_handle(&mut self, msg: Option<MouseEventMessage>) -> Option<MouseEventMessage> {
        match msg {
            Some(MouseEventMessage::Resize) => {
                self.update_resize();
                Some(MouseEventMessage::Resize)
            }
            Some(msg) => Some(msg),
            None => None,
        }
    }
}

impl Drop for MouseEventHandler {
    fn drop(&mut self) {
        self.stop();
    }
}
