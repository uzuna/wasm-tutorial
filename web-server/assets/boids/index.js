import init, { start_boids } from "./pkg/boids.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

// Give the canvas room for all of our cells and a 1px border
// around each of them.
const canvas_webgl = document.getElementById("webgl-canvas");
const ctrl = start_boids(canvas_webgl);

const slider_visual_range = document.getElementById("visual_range");
const value_visual_range = document.getElementById("visual_range_value");

slider_visual_range.oninput = function () {
    let v = this.value / 100.0;
    value_visual_range.innerText = v;
    ctrl.set_visual_range(v);
}


const slider_center_factor = document.getElementById("center_factor");
const value_center_factor = document.getElementById("center_factor_value");

slider_center_factor.oninput = function () {
    let v = this.value / 1000.0;
    value_center_factor.innerText = v;
    ctrl.set_center_factor(v);
}
