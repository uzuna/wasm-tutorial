use std::{cell::RefCell, rc::Rc};

use futures_channel::mpsc::UnboundedSender;
use fxhash::FxHashMap;
use wasm_bindgen::prelude::*;
use web_sys::MouseEvent;

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

#[derive(Debug, Clone, Copy)]
pub struct MouseMessage {
    pub pos: Point,
    pub down: bool,
}

#[derive(Debug, Default)]
struct MouseState {
    offset_c: Point,
    area_c: Point,
    down: bool,
    pos: Point,
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
            down: self.down,
        }
    }
}

pub struct MouseEventHandler {
    canvas: web_sys::HtmlCanvasElement,
    sender: UnboundedSender<MouseMessage>,
    closures: FxHashMap<String, Closure<dyn FnMut(MouseEvent)>>,
    state: Rc<RefCell<MouseState>>,
}

impl MouseEventHandler {
    pub fn new(canvas: web_sys::HtmlCanvasElement, tx: UnboundedSender<MouseMessage>) -> Self {
        let state = Rc::new(RefCell::new(MouseState::from_canvas(&canvas)));
        Self {
            canvas,
            sender: tx,
            closures: FxHashMap::default(),
            state,
        }
    }

    pub fn start(&mut self) {
        let event_type = "mousedown";
        let state = self.state.clone();
        let mut tx = self.sender.clone();
        let mouse_down = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut state = state.borrow_mut();
            state.update_pos(event.client_x(), event.client_y());
            state.mouse_down();
            tx.start_send(state.msg()).unwrap();
        }) as Box<dyn FnMut(_)>);
        self.canvas
            .add_event_listener_with_callback(event_type, mouse_down.as_ref().unchecked_ref())
            .unwrap();
        self.closures.insert(event_type.to_string(), mouse_down);

        let event_type = "mouseup";
        let state = self.state.clone();
        let mut tx = self.sender.clone();
        let mouse_up = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut state = state.borrow_mut();
            state.update_pos(event.client_x(), event.client_y());
            state.mouse_up();
            tx.start_send(state.msg()).unwrap();
        }) as Box<dyn FnMut(_)>);
        self.canvas
            .add_event_listener_with_callback(event_type, mouse_up.as_ref().unchecked_ref())
            .unwrap();
        self.closures.insert(event_type.to_string(), mouse_up);

        let event_type = "mousemove";
        let state = self.state.clone();
        let mut tx = self.sender.clone();
        let mouse_move = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let mut state = state.borrow_mut();
            state.update_pos(event.client_x(), event.client_y());
            tx.start_send(state.msg()).unwrap();
        }) as Box<dyn FnMut(_)>);
        self.canvas
            .add_event_listener_with_callback(event_type, mouse_move.as_ref().unchecked_ref())
            .unwrap();
        self.closures.insert(event_type.to_string(), mouse_move);
    }

    #[allow(dead_code)]
    pub fn stop(&mut self) {
        for (event_type, closure) in self.closures.drain() {
            self.canvas
                .remove_event_listener_with_callback(
                    event_type.as_str(),
                    closure.as_ref().unchecked_ref(),
                )
                .unwrap();
        }
    }

    pub fn forget(self) {
        for (_event_type, closure) in self.closures {
            closure.forget();
        }
    }
}
