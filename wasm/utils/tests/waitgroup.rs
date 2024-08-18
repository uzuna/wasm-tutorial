//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;

use std::{assert, rc::Rc, sync::atomic::AtomicU32};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_test::*;

use wasm_utils::waitgroup::WaitGroup;

wasm_bindgen_test_configure!(run_in_browser);

// 同期での待ち合わせのテスト。同期ループみたいなもので確認するケースを想定
#[wasm_bindgen_test]
async fn test_wait_sync() -> std::result::Result<(), JsValue> {
    let wg = WaitGroup::new();
    assert!(wg.is_finished());
    let w1 = wg.add();
    assert!(!wg.is_finished());
    drop(w1);
    assert!(wg.is_finished());

    Ok(())
}

// 非同期での待ち合わせのテスト
#[wasm_bindgen_test]
async fn test_wait_async() -> std::result::Result<(), JsValue> {
    let wg = WaitGroup::new();

    // 2タスクを追加
    let w1 = wg.add();
    let w2 = wg.add();
    assert!(!wg.is_finished());

    // タスク終了時に加算するカウンタ
    let counter = Rc::new(AtomicU32::new(0));
    let c1 = counter.clone();
    let c2 = counter.clone();

    // タスク開始
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(10).await;
        c1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        drop(w1);
    });
    spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(20).await;
        c2.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        drop(w2);
    });

    // タスク前は完了が0で、待ちの後には2になる
    assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 0);
    wg.wait().await;
    assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 2);

    Ok(())
}
