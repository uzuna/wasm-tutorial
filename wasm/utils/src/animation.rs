use std::{
    borrow::Borrow,
    cell::RefCell,
    rc::{Rc, Weak},
    sync::atomic::{AtomicBool, Ordering::Relaxed},
};

use wasm_bindgen::prelude::*;

use crate::error::Result;

// アニメーションフレームのコールバック
// タイムスタンプが渡され、次のアニメーションフレームのIDを返す
pub type AnimationCallback = dyn FnMut(f64) -> Result<i32>;
// RustのクロージャをRust、Js両方から呼び出せる型
pub type RequestAnimationFrameClosure = Closure<AnimationCallback>;

// 次のアニメーションフレームをリクエストする
fn request_animation_frame(closure: &RequestAnimationFrameClosure) -> i32 {
    let window = web_sys::window().expect("Failed to get window");
    window
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .expect("Failed to request animation frame")
}

// 再生リクエストをキャンセル
fn cancel_animation_frame(handle: i32) {
    let window = web_sys::window().expect("Failed to get window");
    window
        .cancel_animation_frame(handle)
        .expect("Failed to cancel animation frame");
}

#[derive(Debug, Clone)]
pub struct AnimationLoop {
    animation_ctx: Rc<RefCell<Option<i32>>>,
    closure_ctx: Rc<Closure<dyn FnMut(f64) -> Result<i32>>>,
    document_timeline: f64,
    performance_start: f64,
}

impl AnimationLoop {
    pub fn new(mut callback: impl FnMut(f64) -> Result<()> + 'static) -> Self {
        let a_ctx = Rc::new(RefCell::new(None));
        let a_ctx_clone = a_ctx.clone();
        let closure = Rc::new_cyclic(|this: &Weak<_>| {
            // &Weak -> Weak
            let this = this.clone();
            RequestAnimationFrameClosure::new(move |timestamp_msec| {
                callback(timestamp_msec)?;

                // set next frame
                let this = this.upgrade().unwrap();
                let handle = request_animation_frame(&this);
                *a_ctx_clone.borrow_mut() = Some(handle);
                Ok(handle)
            })
        });

        Self {
            animation_ctx: a_ctx,
            closure_ctx: closure,
            document_timeline: 0.0,
            performance_start: 0.0,
        }
    }

    pub fn start(&mut self) {
        self.document_timeline = web_sys::window()
            .expect("Failed to get window")
            .document()
            .expect("Failed to get performance")
            .timeline()
            .current_time()
            .expect("Failed to get current time");
        self.performance_start = web_sys::window()
            .expect("Failed to get window")
            .performance()
            .expect("Failed to get performance")
            .now();
        *self.animation_ctx.borrow_mut() = Some(request_animation_frame(self.closure_ctx.borrow()));
    }

    pub fn cancel(&self) -> Result<()> {
        if let Some(handle) = self.animation_ctx.borrow_mut().take() {
            cancel_animation_frame(handle);
            Ok(())
        } else {
            Err(JsError::new("Animation Frame is not started"))
        }
    }

    /// アニメーションクロージャは構造体と寿命が紐付いているため、構造体を破棄した後も再生するためにはforgetが必要
    pub fn forget(&self) {
        std::mem::forget(self.closure_ctx.clone());
    }

    fn is_started(&self) -> bool {
        self.animation_ctx.borrow_mut().is_some()
    }
}

/// AnimationLoopに再生、停止のインタラクションを追加
pub struct PlayStopButton {
    element: web_sys::HtmlButtonElement,
    play: bool,
    animation_loop: AnimationLoop,
    playing: Rc<RefCell<AtomicBool>>,
}

impl PlayStopButton {
    pub fn new(element: web_sys::HtmlButtonElement, animation_loop: AnimationLoop) -> Self {
        let playing = Rc::new(RefCell::new(AtomicBool::new(false)));
        Self::new_with_flag(element, animation_loop, playing)
    }

    pub fn new_with_flag(
        element: web_sys::HtmlButtonElement,
        animation_loop: AnimationLoop,
        playing: Rc<RefCell<AtomicBool>>,
    ) -> Self {
        let play = animation_loop.is_started();
        playing.borrow_mut().store(play, Relaxed);
        let s = Self {
            element,
            play,
            animation_loop,
            playing,
        };
        s.set_text();
        s
    }

    fn set_text(&self) {
        self.element
            .set_text_content(Some(if self.play { "Stop" } else { "Play" }));
    }

    pub fn play(&mut self) {
        self.play = true;
        self.playing.borrow_mut().store(true, Relaxed);
        self.animation_loop.start();
        self.set_text();
    }

    pub fn stop(&mut self) -> Result<()> {
        self.play = false;
        self.playing.borrow_mut().store(false, Relaxed);
        self.set_text();
        self.animation_loop.cancel()
    }

    pub fn toggle(&mut self) -> Result<()> {
        if self.play {
            self.stop()?;
        } else {
            self.play();
        };
        Ok(())
    }

    pub fn start(self) -> PlayAnimaionContext {
        let ctx = Rc::new(RefCell::new(self));
        let ctx_clone = ctx.clone();
        let closure = Closure::wrap(Box::new(move || {
            let mut this = ctx_clone.borrow_mut();
            if this.play {
                let _ = this.stop();
            } else {
                this.play();
            }
        }) as Box<dyn FnMut()>);
        ctx.borrow_mut()
            .element
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        PlayAnimaionContext { ctx, closure }
    }

    pub fn flag(&self) -> Rc<RefCell<AtomicBool>> {
        self.playing.clone()
    }

    fn forget(&self) {
        self.animation_loop.forget();
    }
}

#[wasm_bindgen]
pub struct PlayAnimaionContext {
    ctx: Rc<RefCell<PlayStopButton>>,
    closure: Closure<dyn FnMut()>,
}

impl PlayAnimaionContext {
    pub fn forget(self) {
        let Self { ctx, closure } = self;
        std::mem::forget(closure);
        ctx.borrow_mut().forget();
    }
}
