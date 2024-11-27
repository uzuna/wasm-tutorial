use std::{
    borrow::Borrow,
    cell::RefCell,
    rc::{Rc, Weak},
    sync::atomic::{AtomicU64, Ordering},
};

use wasm_bindgen::prelude::*;

use crate::{error::Result, util::get_window};

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
}

#[cfg(feature = "input")]
pub mod ctrl {
    use futures_channel::mpsc;
    use futures_util::StreamExt;
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::atomic::{AtomicBool, Ordering},
    };

    use crate::{
        error::*,
        input::{button::SubmitBtn, InputIdent},
    };

    use super::AnimationLoop;

    #[derive(Debug, Clone)]
    pub enum AnimationCtrl {
        Playing(bool),
    }

    impl InputIdent for AnimationCtrl {
        fn id(&self) -> &'static str {
            match self {
                Self::Playing(_) => "play-pause",
            }
        }
    }

    /// AnimationLoopに再生、停止のインタラクションを追加
    pub struct PlayStopButton {
        btn: SubmitBtn<AnimationCtrl>,
        animation_loop: AnimationLoop,
        playing: Rc<RefCell<AtomicBool>>,
    }

    impl PlayStopButton {
        pub fn new(animation_loop: AnimationLoop, initial_value: bool) -> Result<Self> {
            let btn = SubmitBtn::new(AnimationCtrl::Playing(initial_value))?;
            let playing = Rc::new(RefCell::new(AtomicBool::new(initial_value)));
            let s = Self {
                btn,
                animation_loop,
                playing,
            };
            s.set_text();
            Ok(s)
        }

        fn set_text(&self) {
            let current = self.playing.borrow().load(Ordering::Relaxed);
            self.btn
                .set_text(Some(if current { "Stop" } else { "Play" }));
        }

        pub fn set_play(&mut self, play: bool) {
            let current = self.playing.borrow().swap(play, Ordering::Relaxed);
            if current != play {
                if play {
                    self.animation_loop.start();
                } else {
                    self.animation_loop.cancel().unwrap();
                }
                self.set_text();
            }
        }

        pub fn start(mut self, mut tx: mpsc::Sender<AnimationCtrl>) -> Result<()> {
            let (tx_inner, mut rx) = mpsc::channel(1);
            self.btn.start(tx_inner).unwrap();
            wasm_bindgen_futures::spawn_local(async move {
                while let Some(AnimationCtrl::Playing(playing)) = rx.next().await {
                    self.set_play(playing);
                    tx.try_send(AnimationCtrl::Playing(playing)).unwrap();
                }
            });
            Ok(())
        }
    }
}

/// 非同期の中でrequest animation frameの周期を待つTicker
pub struct AnimationTicker {
    timestamp: Rc<AtomicU64>,
}

impl AnimationTicker {
    /// 次のアニメーションフレームを待つ
    pub async fn tick(&mut self) -> Result<f64> {
        let instant = AnimationInstant::new(self.timestamp.clone());
        instant.await
    }

    /// 最後のタイムスタンプを取得
    pub fn last_timestamp(&self) -> f64 {
        f64::from_bits(self.timestamp.load(Ordering::Relaxed))
    }
}

impl Default for AnimationTicker {
    fn default() -> Self {
        Self {
            timestamp: Rc::new(AtomicU64::new(0)),
        }
    }
}

// requestAnimationFrameを待つFutureの実装
struct AnimationInstant {
    closure: Option<Closure<dyn FnMut(f64)>>,
    handle: Option<i32>,
    timestamp: Rc<AtomicU64>,
}

impl AnimationInstant {
    fn new(timestamp: Rc<AtomicU64>) -> Self {
        Self {
            closure: None,
            handle: None,
            timestamp,
        }
    }

    fn cancel(&mut self) {
        if let Some(handle) = self.handle.take() {
            cancel_animation_frame(handle);
        }
    }
}

impl std::future::Future for AnimationInstant {
    type Output = Result<f64>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Self::Output> {
        use std::borrow::BorrowMut;
        // wakerが呼ばれたら基本的にはタスクが終了しているはず
        if let Some(_handle) = self.handle.take() {
            let ts = f64::from_bits(self.timestamp.load(Ordering::Relaxed));
            std::task::Poll::Ready(Ok(ts))
        } else {
            // await callされたたらタスクを開始
            let waker = cx.waker().clone();
            let mut ts = self.timestamp.clone();
            let closure = Closure::wrap(Box::new(move |timestamp| {
                ts.borrow_mut()
                    .store(f64::to_bits(timestamp), Ordering::Relaxed);
                waker.wake_by_ref();
            }) as Box<dyn FnMut(f64)>);
            self.handle = Some(request_animation_frame_inner(&closure)?);
            self.closure = Some(closure);
            std::task::Poll::Pending
        }
    }
}

impl Drop for AnimationInstant {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn request_animation_frame_inner(closure: &Closure<dyn FnMut(f64)>) -> Result<i32> {
    get_window()?
        .request_animation_frame(closure.as_ref().unchecked_ref())
        .map_err(|_| JsError::new("Failed request animation frame"))
}
