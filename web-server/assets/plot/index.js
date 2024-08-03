import init, { start } from "./pkg/plot.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

const canvas_webgl = document.getElementById("webgl-canvas");
const play_pause_btn = document.getElementById("play-pause");
const context = start(canvas_webgl, play_pause_btn);
