use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
};

use futures_channel::mpsc::UnboundedSender;
use fxhash::FxHashMap;
use wasm_bindgen::prelude::*;
use web_sys::{MouseEvent, WheelEvent};

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

/// モジュール外にマウスとホイールの状態を通知するメッセージ
#[derive(Debug, Clone, Copy)]
pub struct MouseMessage {
    pub pos: Point,
    pub wheel: Wheel,
    pub down: bool,
}

// マウスの状態を保持する構造体
#[derive(Debug, Default)]
struct MouseState {
    offset_c: Point,
    area_c: Point,
    down: bool,
    pos: Point,
    wheel: Wheel,
}

impl MouseState {
    fn new(offset_c: Point, area_c: Point) -> Self {
        Self {
            offset_c,
            area_c,
            ..Default::default()
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

    fn update_pos(&mut self, pos_x: i32, pos_y: i32) {
        // self.down = true;
        let pos = Point {
            x: pos_x as f32,
            y: pos_y as f32,
        };
        self.pos = pos;
    }

    fn update_wheel(&mut self, delta_x: f32, delta_y: f32) {
        self.wheel = Wheel {
            x: delta_x,
            y: delta_y,
        };
    }

    fn mouse_down(&mut self) {
        self.down = true;
    }

    fn mouse_up(&mut self) {
        self.down = false;
    }

    fn msg(&self) -> MouseMessage {
        // マウス座標をOpenGL空間に変換
        let mut mouse_pos = (self.pos - self.offset_c - self.area_c / 2.) / self.area_c * 2.;
        mouse_pos.y = -mouse_pos.y;

        MouseMessage {
            pos: mouse_pos,
            wheel: self.wheel,
            down: self.down,
        }
    }

    fn fetch_msg(&mut self) -> MouseMessage {
        let msg = self.msg();
        // wheelは継続する情報ではないのでリセット
        self.wheel = Wheel::default();
        msg
    }
}

/// マウスイベントを処理する構造体
pub struct MouseEventHandler {
    canvas: web_sys::HtmlCanvasElement,
    sender: UnboundedSender<MouseMessage>,
    mouse_closures: FxHashMap<String, Closure<dyn FnMut(MouseEvent)>>,
    wheel_closures: FxHashMap<String, Closure<dyn FnMut(WheelEvent)>>,
    state: Rc<RefCell<MouseState>>,
}

impl MouseEventHandler {
    pub fn new(canvas: web_sys::HtmlCanvasElement, tx: UnboundedSender<MouseMessage>) -> Self {
        let state = Rc::new(RefCell::new(MouseState::from_canvas(&canvas)));
        Self {
            canvas,
            sender: tx,
            mouse_closures: FxHashMap::default(),
            wheel_closures: FxHashMap::default(),
            state,
        }
    }

    pub fn start(&mut self) {
        // マウスの上げ下げイベントは位置と状態を更新
        self.build_mouse_closure("mousedown", |(state, event)| {
            state.update_pos(event.client_x(), event.client_y());
            state.mouse_down();
        });

        self.build_mouse_closure("mouseup", |(state, event)| {
            state.update_pos(event.client_x(), event.client_y());
            state.mouse_up();
        });

        // マウス移動は移動のみを取得
        self.build_mouse_closure("mousemove", |(state, event)| {
            state.update_pos(event.client_x(), event.client_y());
        });

        // ホイールイベントはホイールの移動量を取得
        self.build_wheel_closure("wheel", |(state, event)| {
            state.update_wheel(event.delta_x() as f32, event.delta_y() as f32);
        });
    }

    // マウスイベントのクロージャを登録
    fn build_mouse_closure(
        &mut self,
        event_type: &str,
        mut f: impl FnMut((&mut RefMut<MouseState>, MouseEvent)) + 'static,
    ) {
        let state = self.state.clone();
        let mut tx = self.sender.clone();
        let clusure = Closure::wrap(Box::new(move |event: MouseEvent| {
            let mut state = state.borrow_mut();
            f((&mut state, event));
            tx.start_send(state.msg()).unwrap();
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
        mut f: impl FnMut((&mut RefMut<MouseState>, WheelEvent)) + 'static,
    ) {
        let state = self.state.clone();
        let mut tx = self.sender.clone();
        let clusure = Closure::wrap(Box::new(move |event: WheelEvent| {
            let mut state = state.borrow_mut();
            f((&mut state, event));
            // ホイールイベントは継続する情報ではないのでリセット
            tx.start_send(state.fetch_msg()).unwrap();
        }) as Box<dyn FnMut(WheelEvent)>);

        self.canvas
            .add_event_listener_with_callback(event_type, clusure.as_ref().unchecked_ref())
            .unwrap();
        self.wheel_closures.insert(event_type.to_string(), clusure);
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

    pub fn forget(self) {
        for (_event_type, closure) in self.mouse_closures {
            closure.forget();
        }
        for (_event_type, closure) in self.wheel_closures {
            closure.forget();
        }
    }
}
