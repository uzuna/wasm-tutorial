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

#[wasm_bindgen_test]
pub fn test_tick() {
    fn input_spaceship() -> Universe {
        let mut universe = Universe::new(6, 6);
        universe.set_cells(&[(1, 2), (2, 3), (3, 1), (3, 2), (3, 3)]);
        universe
    }

    fn expected_spaceship() -> Universe {
        let mut universe = Universe::new(6, 6);
        universe.set_cells(&[(2, 1), (2, 3), (3, 2), (3, 3), (4, 2)]);
        universe
    }

    let mut input_universe = input_spaceship();

    let expected_universe = expected_spaceship();

    input_universe.tick();
    assert!(input_universe.difference(&expected_universe) < 1);
}
