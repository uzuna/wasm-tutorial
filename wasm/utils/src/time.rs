use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

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
