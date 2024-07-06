import { GolBuilder, golstart, webgl_start } from "wasm-game-of-life";

// Give the canvas room for all of our cells and a 1px border
// around each of them.
const canvas = document.getElementById("game-of-life-canvas");
const canvas_webgl = document.getElementById("webgl-canvas");
const width = 64;
const height = 64;
const playPauseButton = document.getElementById("play-pause");
const fps = document.getElementById("fps");
const golb = GolBuilder.new(width, height, canvas, playPauseButton, fps);
golstart(golb);
webgl_start(canvas_webgl);
