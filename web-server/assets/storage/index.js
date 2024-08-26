import init, { start } from "./pkg/wasm_storage.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

var container = document.getElementById("ctrl1-container");
const context = start(container);


// スライダインタラクションの設定
function setup_slider(slider_id, value_id, slide_params) {
    const slider = document.getElementById(slider_id);
    const value_elem = document.getElementById(value_id);
    slider.min = slide_params.min * slide_params.rate;
    slider.max = slide_params.max * slide_params.rate;
    slider.value = slide_params.value * slide_params.rate;
    value_elem.innerText = slider.value / slide_params.rate;
    slider.oninput = function () {
        let v = this.value / slide_params.rate;
        value_elem.innerText = v;
        slide_params.callback(v);
    }
}

const c1_slider = {
    min: 0.0,
    max: 1.0,
    value: function() { 0.5 },
    rate: 100.0,
    callback: (v) => {}
}
setup_slider("c1-test", "c1-test-value", c1_slider);
