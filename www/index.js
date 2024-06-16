import { Universe } from "wasm-game-of-life";

const pre = document.getElementById("game-of-life-canvas");
const universe = Universe.new(64, 32);

// コールバックループ
const renderLoop = () => {
    // テキストエリアにwasm空間からメモリをコピー
    pre.textContent = universe.render();
    // 更新
    universe.tick();

    // ループ再帰
    requestAnimationFrame(renderLoop);
};

// ループ開始トリガー
requestAnimationFrame(renderLoop);
