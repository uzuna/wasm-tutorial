use std::{
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    task::{Context, Poll},
    time::Duration,
};

use futures_util::{stream::FusedStream, Stream};
use wasm_bindgen::prelude::*;

use crate::{error::*, util::get_window};

pub struct Timeout {
    millis: i32,
    id: Option<i32>,
    closure: Option<Closure<dyn FnMut()>>,
}

impl Timeout {
    pub fn new(millis: i32) -> Self {
        Self {
            millis,
            id: None,
            closure: None,
        }
    }

    pub fn cancel(&mut self) {
        if let Some(id) = self.id.take() {
            get_window().unwrap_throw().clear_timeout_with_handle(id);
        }
    }
}

impl Future for Timeout {
    type Output = Result<()>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if let Some(_id) = self.id.take() {
            Poll::Ready(Ok(()))
        } else {
            let waker = cx.waker().clone();
            let closure = Closure::once(move || {
                waker.wake_by_ref();
            });
            let id = get_window()?
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    self.millis,
                )
                .unwrap_throw();
            self.id = Some(id);
            self.closure = Some(closure);
            Poll::Pending
        }
    }
}

impl Drop for Timeout {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub async fn sleep(dur: Duration) -> Result<()> {
    Timeout::new(dur.as_millis() as i32).await
}

/// set_intervalを利用した周期タイマー
pub struct Interval {
    millis: i32,
    id: Option<i32>,
    closure: Option<Closure<dyn FnMut()>>,
    value: Rc<AtomicBool>,
    closed: bool,
}

impl Interval {
    /// set_intervalのラッパー
    pub fn new(millis: i32) -> Self {
        Self {
            millis,
            id: None,
            closure: None,
            value: Rc::new(AtomicBool::new(false)),
            closed: false,
        }
    }

    /// 指定時間ごとに呼び出される周期タイマーを作成する
    pub fn with_duration(dur: Duration) -> Self {
        Self::new(dur.as_millis() as i32)
    }

    pub fn cancel(&mut self) {
        self.closed = true;
        if let Some(id) = self.id.take() {
            get_window().unwrap_throw().clear_interval_with_handle(id);
        }
    }
}

// FutureとStreamの違いは?
// Futureは解決後には破棄される?
impl Stream for Interval {
    type Item = Result<()>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if self.closed {
            Poll::Ready(None)
        } else if self.value.load(Ordering::Relaxed) {
            self.value.store(false, Ordering::Relaxed);
            Poll::Ready(Some(Ok(())))
        } else if self.id.is_some() {
            Poll::Pending
        } else {
            let waker = cx.waker().clone();
            let b = self.value.clone();
            let closure = Closure::new(move || {
                b.store(true, Ordering::Relaxed);
                waker.wake_by_ref();
            });
            let id = get_window()?
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    self.millis,
                )
                .unwrap_throw();
            self.id = Some(id);
            self.closure = Some(closure);
            Poll::Pending
        }
    }
}

impl FusedStream for Interval {
    fn is_terminated(&self) -> bool {
        self.closed
    }
}

impl Drop for Interval {
    fn drop(&mut self) {
        self.cancel();
    }
}
