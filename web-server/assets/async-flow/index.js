import init, { start } from "./pkg/async_flow.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

const canvas_webgl = document.getElementById("webgl-canvas");
start(canvas_webgl);
