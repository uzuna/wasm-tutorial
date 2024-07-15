import init, { start_boids } from "./pkg/boids.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

// Give the canvas room for all of our cells and a 1px border
// around each of them.
const canvas_webgl = document.getElementById("webgl-canvas");
const ctrl = start_boids(canvas_webgl);
console.info(ctrl.param().toJSON());

// スライダインタラクションの設定
function setup_slider(slider_id, value_id, slide_params) {
    const slider = document.getElementById(slider_id);
    const value_elem = document.getElementById(value_id);
    slider.min = slide_params.min;
    slider.max = slide_params.max * slide_params.rate;
    slider.value = slide_params.value * slide_params.rate;
    value_elem.innerText = slider.value / slide_params.rate;
    slider.oninput = function () {
        let v = this.value / slide_params.rate;
        value_elem.innerText = v;
        slide_params.callback(v);
    }
}

const visual_range_slider_params = {
    min: 0.0,
    max: 0.5,
    value: ctrl.param().visual_range,
    rate: 100.0,
    callback: (v) => ctrl.set_visual_range(v)
}
setup_slider("visual_range", "visual_range_value", visual_range_slider_params);

const center_factor_slider_params = {
    min: 0.0,
    max: 0.02,
    value: ctrl.param().center_factor,
    rate: 10000.0,
    callback: (v) => ctrl.set_center_factor(v)
}
setup_slider("center_factor", "center_factor_value", center_factor_slider_params);

const alignment_factor_slider_params = {
    min: 0.0,
    max: 0.1,
    value: ctrl.param().alignment_factor,
    rate: 5000.0,
    callback: (v) => ctrl.set_alignment_factor(v)
}
setup_slider("alignment_factor", "alignment_factor_value", alignment_factor_slider_params);

const avoid_distance_slider_params = {
    min: 0.0,
    max: 0.1,
    value: ctrl.param().avoid_distance,
    rate: 1000.0,
    callback: (v) => ctrl.set_avoid_distance(v)
}
setup_slider("avoid_distance", "avoid_distance_value", avoid_distance_slider_params);

const avoid_factor_slider_params = {
    min: 0.0,
    max: 0.05,
    value: ctrl.param().avoid_factor,
    rate: 10000.0,
    callback: (v) => ctrl.set_avoid_factor(v)
}
setup_slider("avoid_factor", "avoid_factor_value", avoid_factor_slider_params);

const speed_min_slider_params = {
    min: 0.0,
    max: 0.02,
    value: ctrl.param().speed_min,
    rate: 5000.0,
    callback: (v) => ctrl.set_speed_min(v)
}
setup_slider("speed_min", "speed_min_value", speed_min_slider_params);

const speed_max_slider_params = {
    min: 0.005,
    max: 0.02,
    value: ctrl.param().speed_max,
    rate: 10000.0,
    callback: (v) => ctrl.set_speed_max(v)
}
setup_slider("speed_max", "speed_max_value", speed_max_slider_params);

