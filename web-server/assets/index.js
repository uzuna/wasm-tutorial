import init, { GolBuilder, golstart, webgl_start, webgl_interaction } from "./wgol/wasm_game_of_life.js";

// bundlerを伴わない場合はinitが必要
// https://rustwasm.github.io/docs/wasm-bindgen/examples/without-a-bundler.html
await init();

// Give the canvas room for all of our cells and a 1px border
// around each of them.
const canvas = document.getElementById("game-of-life-canvas");
const canvas_webgl = document.getElementById("webgl-canvas");
const canvas_interaction = document.getElementById("webgl-interaction");
const width = 64;
const height = 64;
const playPauseButton = document.getElementById("play-pause");
const fps = document.getElementById("fps");
const golb = GolBuilder.new(width, height, canvas, playPauseButton, fps);
golstart(golb);
webgl_start(canvas_webgl);
webgl_interaction(canvas_interaction);
