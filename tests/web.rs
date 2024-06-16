//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;

use std::assert_eq;

use wasm_bindgen_test::*;
use wasm_game_of_life::Universe;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_pass() {
    let uni = Universe::new(64, 64);
    assert_eq!(uni.render().len(), 12352);
}
