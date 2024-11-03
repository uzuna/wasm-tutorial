//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;

use std::assert;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen_test::*;

use wasm_utils::{time::*, util::get_performance};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_timeout() -> std::result::Result<(), JsValue> {
    let p = get_performance().unwrap_throw();
    let now = p.now();
    let timeout = Timeout::new(10);
    timeout.await?;
    let elapsed = p.now() - now;
    assert!(elapsed >= 10.0);
    Ok(())
}

#[wasm_bindgen_test]
async fn test_interval() -> std::result::Result<(), JsValue> {
    use futures_util::stream::StreamExt;
    let p = get_performance().unwrap_throw();
    let now = p.now();
    let mut interval = Interval::new(10);
    let mut count = 0;
    loop {
        interval.next().await.unwrap();
        count += 1;
        if count >= 10 {
            break;
        }
    }
    let elapsed = p.now() - now;
    assert!(elapsed >= 100.0);
    assert!(count == 10);
    Ok(())
}
