use std::{
    borrow::Borrow,
    cell::RefCell,
    rc::{Rc, Weak},
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
        }
    }

    pub fn start(&self) {
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
}
