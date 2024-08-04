//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;

use std::assert_eq;

use wasm_bindgen::{prelude::*, JsError};
use wasm_bindgen_test::*;
use wasm_utils::{animation::AnimationLoop, error, info};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_console_out() -> std::result::Result<(), JsValue> {
    info!("test_console_out info");
    error!("test_console_out error");
    Ok(())
}