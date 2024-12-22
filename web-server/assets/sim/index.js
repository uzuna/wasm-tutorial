import init, { start, WindowSize } from "./pkg/sim_view.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

const canvas_webgl = document.getElementById("webgl-canvas");
const ws = WindowSize.default();
start(canvas_webgl, ws);
