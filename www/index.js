import { Universe, GolBuilder, golstart } from "wasm-game-of-life";

// wasmのメモリ空間に直接アクセス
import { memory } from "wasm-game-of-life/wasm_game_of_life_bg";

const CELL_SIZE = 5; // px
const GRID_COLOR = "#CCCCCC";
const DEAD_COLOR = "#FFFFFF";
const ALIVE_COLOR = "#000000";


// Give the canvas room for all of our cells and a 1px border
// around each of them.
const canvas = document.getElementById("game-of-life-canvas");
const width = 64;
const height = 64;
const golb = GolBuilder.new(width, height, canvas);
golstart(golb);

const ctx = canvas.getContext('2d');

const playPauseButton = document.getElementById("play-pause");

// フレーム識別子を保持する変数。null以外なら再生中と判断できる
let animationId = null;

// ドラッグ中かどうかを保持する変数
let dragging = false;

const isPaused = () => {
    return animationId === null;
};

const play = () => {
    playPauseButton.textContent = "⏸";
    renderLoop();
};

const pause = () => {
    playPauseButton.textContent = "▶";
    cancelAnimationFrame(animationId);
    animationId = null;
};

playPauseButton.addEventListener("click", event => {
    if (isPaused()) {
        play();
    } else {
        pause();
    }
});

const fps = new class {
    constructor() {
        this.fps = document.getElementById("fps");
        this.frames = [];
        this.lastFrameTimeStamp = performance.now();
    }

    render() {
        // Convert the delta time since the last frame render into a measure
        // of frames per second.
        const now = performance.now();
        const delta = now - this.lastFrameTimeStamp;
        this.lastFrameTimeStamp = now;
        const fps = 1 / delta * 1000;

        // Save only the latest 100 timings.
        this.frames.push(fps);
        if (this.frames.length > 100) {
            this.frames.shift();
        }

        // Find the max, min, and mean of our 100 latest timings.
        let min = Infinity;
        let max = -Infinity;
        let sum = 0;
        for (let i = 0; i < this.frames.length; i++) {
            sum += this.frames[i];
            min = Math.min(this.frames[i], min);
            max = Math.max(this.frames[i], max);
        }
        let mean = sum / this.frames.length;

        // Render the statistics.
        this.fps.textContent = `
  Frames per Second:
           latest = ${Math.round(fps)}
  avg of last 100 = ${Math.round(mean)}
  min of last 100 = ${Math.round(min)}
  max of last 100 = ${Math.round(max)}
  `.trim();
    }
};

const renderLoop = () => {
    // breakpointの設定は `debugger;`でできる
    // debugger;

    fps.render(); //new
    drawGrid();
    animationId = requestAnimationFrame(renderLoop);
};

// 文字通りグリットの描画
const drawGrid = () => {
    ctx.beginPath();
    ctx.strokeStyle = GRID_COLOR;

    // Vertical lines.
    for (let i = 0; i <= width; i++) {
        ctx.moveTo(i * (CELL_SIZE + 1) + 1, 0);
        ctx.lineTo(i * (CELL_SIZE + 1) + 1, (CELL_SIZE + 1) * height + 1);
    }

    // Horizontal lines.
    for (let j = 0; j <= height; j++) {
        ctx.moveTo(0, j * (CELL_SIZE + 1) + 1);
        ctx.lineTo((CELL_SIZE + 1) * width + 1, j * (CELL_SIZE + 1) + 1);
    }

    ctx.stroke();
};

// メモリ空間のインデックスを取得
const getIndex = (row, column) => {
    return row * width + column;
};

// Cellを1bitで表現しているので、そのbitが立っているかどうかを取得
const bitIsSet = (n, arr) => {
    const byte = Math.floor(n / 8);
    const mask = 1 << (n % 8);
    return (arr[byte] & mask) === mask;
};

// canvas.addEventListener("mousedown", event => {
//     dragging = true;
// });

// canvas.addEventListener("mouseup", event => {
//     dragging = false;
// });

// canvas.addEventListener("mouseleave", event => {
//     dragging = false;
// });

// canvas.addEventListener("mousemove", event => {
//     if (dragging) {
//         const boundingRect = canvas.getBoundingClientRect();

//         const scaleX = canvas.width / boundingRect.width;
//         const scaleY = canvas.height / boundingRect.height;

//         const canvasLeft = (event.clientX - boundingRect.left) * scaleX;
//         const canvasTop = (event.clientY - boundingRect.top) * scaleY;

//         const row = Math.min(Math.floor(canvasTop / (CELL_SIZE + 1)), height - 1);
//         const col = Math.min(Math.floor(canvasLeft / (CELL_SIZE + 1)), width - 1);
//         universe.toggle_cell(row, col);

//         drawGrid();
//         drawCells();
//     }
// });



drawGrid();
play();
