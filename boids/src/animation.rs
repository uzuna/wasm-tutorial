use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::prelude::*;

// アニメーションフレームのコールバック
// タイムスタンプが渡され、次のアニメーションフレームのIDを返す
pub type AnimationCallback = dyn FnMut(f64) -> Result<i32, JsValue>;
// RustのクロージャをRust、Js両方から呼び出せる型
pub type RequestAnimationFrameClosure = Closure<AnimationCallback>;

// 次のアニメーションフレームをリクエストする
fn request_animation_frame(closure: &RequestAnimationFrameClosure) -> Result<i32, JsValue> {
    let window = web_sys::window().unwrap();
    window.request_animation_frame(closure.as_ref().unchecked_ref())
}

// 再生リクエストをキャンセル
fn cancel_animation_frame(handle: i32) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    window.cancel_animation_frame(handle)
}

#[derive(Debug, Clone)]
pub struct AnimationLoop {
    animation_ctx: Rc<RefCell<Option<i32>>>,
    closure_ctx: Rc<RefCell<Option<RequestAnimationFrameClosure>>>,
}

impl AnimationLoop {
    pub fn new(mut callback: impl FnMut(f64) -> Result<(), JsValue> + 'static) -> Self {
        let a_ctx = Rc::new(RefCell::new(None));
        let a_ctx_clone = a_ctx.clone();
        let c = Rc::new(RefCell::new(None));
        let c_clone = c.clone();
        *c.borrow_mut() =
            Some(RequestAnimationFrameClosure::new(
                move |timestamp_msec| match callback(timestamp_msec) {
                    Ok(_) => {
                        let res = request_animation_frame(c_clone.borrow().as_ref().unwrap());
                        match res {
                            Ok(handle) => {
                                *a_ctx_clone.borrow_mut() = Some(handle);
                                Ok(handle)
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(JsValue::from_str(&format!("{:?}", e))),
                },
            ));

        Self {
            animation_ctx: a_ctx,
            closure_ctx: c,
        }
    }

    pub fn start(&self) -> Result<(), JsValue> {
        *self.animation_ctx.borrow_mut() = Some(request_animation_frame(
            self.closure_ctx.borrow().as_ref().unwrap(),
        )?);
        Ok(())
    }

    pub fn cancel(&self) -> Result<(), JsValue> {
        cancel_animation_frame(*self.animation_ctx.borrow().as_ref().unwrap())
    }
}
