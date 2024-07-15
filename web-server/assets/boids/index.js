import init, { start_boids } from "./pkg/boids.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

// Give the canvas room for all of our cells and a 1px border
// around each of them.
const canvas_webgl = document.getElementById("webgl-canvas");
start_boids(canvas_webgl);
