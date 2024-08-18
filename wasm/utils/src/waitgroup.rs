//! 非同期呼び出し結果の待ち合わせを行うための機能を提供するモジュールです。
//!
//! refer:
//! gloo-timers: https://docs.rs/gloo-timers/0.3.0/gloo_timers/future/index.html
//! waitgroup: https://docs.rs/waitgroup/0.1.2/waitgroup/index.html

use std::{
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::atomic::AtomicU32,
    task::{Context, Poll},
};

use futures_channel::mpsc;
use futures_util::{stream::StreamFuture, StreamExt};

/// 非同期処理の待ち合わせを行うための構造体です。
pub struct WaitGroup {
    // 待っているワーカの数
    count: Rc<AtomicU32>,
    // pollを待つためのチャンネル
    rx: mpsc::Receiver<()>,
    // ワーカーからの通知を行うためのチャンネル
    tx: mpsc::Sender<()>,
}

impl Default for WaitGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl WaitGroup {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1);
        WaitGroup {
            count: Rc::new(AtomicU32::new(0)),
            rx,
            tx,
        }
    }

    /// ワーカを追加します。
    pub fn add(&self) -> Worker {
        self.count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Worker {
            count: self.count.clone(),
            tx: self.tx.clone(),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.count.load(std::sync::atomic::Ordering::Relaxed) == 0
    }

    /// 現在の待ちワーカー数
    pub fn count(&self) -> u32 {
        self.count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// 全てのワーカーが終了するまで待ちます。
    pub fn wait(self) -> WaitGroupFuture {
        WaitGroupFuture {
            rx: self.rx.into_future(),
        }
    }
}

/// Futureで待ち合わせを行うための構造体
pub struct WaitGroupFuture {
    rx: StreamFuture<futures_channel::mpsc::Receiver<()>>,
}

impl Future for WaitGroupFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Future::poll(Pin::new(&mut self.rx), cx).map(|_| ())
    }
}

/// 実行中のワーカーを表す構造体。drop時にワーカー数を減らします。
pub struct Worker {
    count: Rc<AtomicU32>,
    tx: mpsc::Sender<()>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        if self
            .count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
            == 0
        {
            let _ = self.tx.clone().try_send(());
        }
    }
}
