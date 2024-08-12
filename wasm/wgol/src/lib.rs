mod error;
mod utils;
mod webgl;

use fixedbitset::FixedBitSet;
use gloo_net::{
    http::Request,
    websocket::{futures::WebSocket, Message},
};
use gloo_timers::future::TimeoutFuture;
use js_sys::Math::random;
use std::{cell::RefCell, fmt, rc::Rc};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use wasm_bindgen::prelude::*;
use wasm_utils::animation::AnimationLoop;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, WebGl2RenderingContext as gl};
use webgl::{
    camera::{Camera, ViewMatrix},
    interaction::ParticleControl,
};

use crate::error::Result;

const GRID_COLOR: &str = "#CCCCCC";

#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[macro_export]
macro_rules! error {
    ( $( $t:tt )* ) => {
        web_sys::console::error_1(&format!( $( $t )* ).into());
    }
}

pub fn jserror(e: JsError) {
    web_sys::console::error_1(&JsValue::from(e));
}

/// ライフゲームのビルダー
/// 複雑な引数を渡すテスト
#[wasm_bindgen]
pub struct GolBuilder {
    width: u32,
    height: u32,
    cell_size: u32,
    canvas: web_sys::HtmlCanvasElement,
    play_button: web_sys::HtmlButtonElement,
    fps: web_sys::HtmlElement,
}

/// 関数をこう飽きする場合はimplにwasm_bindgenをつけてpubにする
#[wasm_bindgen]
impl GolBuilder {
    pub fn new(
        width: u32,
        height: u32,
        canvas: web_sys::HtmlCanvasElement,
        play_button: web_sys::HtmlButtonElement,
        fps: web_sys::HtmlElement,
    ) -> GolBuilder {
        GolBuilder {
            width,
            height,
            cell_size: 5,
            canvas,
            play_button,
            fps,
        }
    }

    // Universeを生成する
    fn build(&self) -> Universe {
        // set canvas size
        self.canvas.set_width((self.width + 1) * self.cell_size);
        self.canvas.set_height((self.height + 1) * self.cell_size);
        Universe::new(self.width, self.height)
    }

    // click event listenerを作る
    // canvasにクロージャを設定して、クリックされたセルの状態をchannel経由で変更する
    fn gol(self, sender: UnboundedSender<(CellControl, Point)>) {
        let ue: UniEventHandler = UniEventHandler {
            cell_size: self.cell_size,
            canvas: self.canvas,
        };

        let ctx = Rc::new(RefCell::new(ue));
        let ctx_clone = Rc::clone(&ctx);
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let x = event.offset_x() as u32 / (ctx_clone.borrow().cell_size + 1);
            let y = event.offset_y() as u32 / (ctx_clone.borrow().cell_size + 1);
            log!("click: ({}, {})", x, y);
            sender.send((CellControl::Toggle, Point { x, y })).unwrap();
        }) as Box<dyn FnMut(_)>);
        ctx.borrow()
            .canvas
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();

        // closureはevent_listenerに渡したので、dropさせない
        closure.forget();
    }
}

// clock event listener向けの変数保持
#[wasm_bindgen]
pub struct UniEventHandler {
    cell_size: u32,
    canvas: web_sys::HtmlCanvasElement,
}

/// セルの状態を示す
#[wasm_bindgen]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Dead = 0,
    Alive = 1,
}

impl Cell {
    fn toggle(&mut self) -> &mut Self {
        *self = match *self {
            Cell::Dead => Cell::Alive,
            Cell::Alive => Cell::Dead,
        };
        self
    }

    fn bool(&self) -> bool {
        match *self {
            Cell::Dead => false,
            Cell::Alive => true,
        }
    }
}

impl From<Cell> for bool {
    #[inline]
    fn from(cell: Cell) -> Self {
        match cell {
            Cell::Dead => false,
            Cell::Alive => true,
        }
    }
}

impl From<bool> for Cell {
    fn from(b: bool) -> Self {
        if b {
            Cell::Alive
        } else {
            Cell::Dead
        }
    }
}

/// ライフゲームの空間を示す
#[wasm_bindgen]
#[derive(Debug)]
pub struct Universe {
    width: u32,
    height: u32,
    cells: FixedBitSet,
}

/// アトリビュートがなければJS側には公開されない
#[wasm_bindgen]
impl Universe {
    /// 大きさを指定して新しいインスタンスを生成する
    pub fn new(width: u32, height: u32) -> Universe {
        utils::set_panic_hook();
        Universe::new_inner(width, height, |i| {
            if i % 2 == 0 || i % 7 == 0 {
                Cell::Alive
            } else {
                Cell::Dead
            }
        })
    }

    /// ランダムな状態で新しいインスタンスを生成する
    pub fn with_random(width: u32, height: u32) -> Universe {
        // stack trace表示に必要。ここで呼ぶ必要があるかは不明...
        utils::set_panic_hook();
        Universe::new_inner(width, height, |_| {
            if random() > 0.5 {
                Cell::Alive
            } else {
                Cell::Dead
            }
        })
    }

    fn new_inner(width: u32, height: u32, rule: impl Fn(usize) -> Cell) -> Universe {
        let size = (width * height) as usize;
        let mut cells = FixedBitSet::with_capacity(size);
        for i in 0..size {
            cells.set(i, rule(i).into());
        }

        log!("Universe created: {}", size);

        Universe {
            width,
            height,
            cells,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// セル配列へのポインタを返す
    pub fn cells(&self) -> *const usize {
        self.cells.as_slice().as_ptr()
    }

    /// すべてのセルを文字列で表現して返す
    pub fn render(&self) -> String {
        self.to_string()
    }

    /// 更新関数
    pub fn tick(&mut self) {
        // let _timer = Timer::new("Universe::tick");
        let mut next = self.cells.clone();
        for row in 0..self.height {
            for col in 0..self.width {
                let idx = self.get_index(row, col);
                let cell = self.cells[idx];
                let live_neighbors = self.live_neighbor_count(row, col);

                next.set(
                    idx,
                    match (cell, live_neighbors) {
                        (true, x) if x < 2 => false,
                        (true, 2) | (true, 3) => true,
                        (true, x) if x > 3 => false,
                        (false, 3) => true,
                        (otherwise, _) => otherwise,
                    },
                );
            }
        }

        self.cells = next;
    }

    // 特定のセルの状態を取得する
    fn get_index(&self, row: u32, column: u32) -> usize {
        (row * self.width + column) as usize
    }

    // 指定セル周辺の行き生存セルの数を返す
    fn live_neighbor_count(&self, row: u32, column: u32) -> u8 {
        let mut count = 0;
        for delta_row in [self.height - 1, 0, 1].iter().cloned() {
            for delta_col in [self.width - 1, 0, 1].iter().cloned() {
                if delta_row == 0 && delta_col == 0 {
                    continue;
                }

                let neighbor_row = (row + delta_row) % self.height;
                let neighbor_col = (column + delta_col) % self.width;
                let idx = self.get_index(neighbor_row, neighbor_col);
                count += self.cells[idx] as u8;
            }
        }
        count
    }

    pub fn difference(&self, other: &Universe) -> usize {
        self.cells.difference_count(&other.cells)
    }

    /// 指定セルの状態を反転する
    pub fn toggle_cell(&mut self, row: u32, column: u32) {
        let idx = self.get_index(row, column);
        let cell = *Cell::from(self.cells[idx]).toggle();
        self.cells.set(idx, cell.into());
        log!("toggle_cell: [{}, {}] = {:?}", row, column, cell);
    }
}

impl fmt::Display for Universe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for row in 0..self.height {
            for col in 0..self.width {
                let idx = self.get_index(row, col);
                let cell = self.cells[idx];
                let symbol = if cell == Cell::Dead.bool() {
                    '◻'
                } else {
                    '◼'
                };
                write!(f, "{}", symbol)?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

impl Universe {
    /// 幅を設定する。(u32, u32)はWASMの制約により使えないのでwasm_bindgenを使わない
    pub fn set_cells(&mut self, cells: &[(u32, u32)]) {
        for (row, col) in cells.iter().cloned() {
            let idx = self.get_index(row, col);
            self.cells.set(idx, Cell::Alive.bool());
        }
    }
}

pub struct Timer<'a> {
    name: &'a str,
}

impl<'a> Timer<'a> {
    pub fn new(name: &'a str) -> Timer<'a> {
        web_sys::console::time_with_label(name);
        Timer { name }
    }
}

impl<'a> Drop for Timer<'a> {
    fn drop(&mut self) {
        web_sys::console::time_end_with_label(self.name);
    }
}

/// WASMのエントリポイント
///
/// 構造体を戻すような使い方をすると、ライフタイムが不明でevent callbackの設定が難しい
/// 実行プロセス全体を関数に閉じ込めたほうが取り回ししやすい
#[wasm_bindgen]
pub fn golstart(gb: GolBuilder) -> Result<()> {
    // JS側の指示はchannel経由で受け取る
    let (sender, mut recv_p, mut recv_c) = Sender::new();

    // UniverseをRcでラップして、非同期taskからアクセスできるようにする
    let uni = Rc::new(RefCell::new(gb.build()));

    // アニメーション更新クロージャ
    // 開始停止が難しいので、良いラップ方法を考えたい。非同期タスクとして見るのが良い?
    let closure = Rc::new(RefCell::new(None));
    // 描画処理
    let drawer = Drawer::default();

    let context = gb
        .canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()
        .unwrap();
    let play_btn = gb.play_button.clone();
    let mut fps = Fps::new(gb.fps.clone());

    gb.gol(sender.c_ctrl.clone());

    // play/pause を制御するanimationIdを保持する変数
    // callbackによる仮面更新に動悸した再生と、cancelAnimationFrameによる停止ができる
    let p = Rc::new(RefCell::new(None));

    // チャンネル経由でplay/pause操作する
    let p_ctrl = p.clone();
    let cls_ctrl = closure.clone();
    let uni_ctrl = uni.clone();
    wasm_bindgen_futures::spawn_local(async move {
        loop {
            tokio::select! {
                Some((ctrl, point)) = recv_c.recv() => {
                    match ctrl {
                        CellControl::Alive => {
                            uni_ctrl.borrow_mut().cells.set(uni_ctrl.borrow().get_index(point.y, point.x), Cell::Alive.into());
                        }
                        CellControl::Dead => {
                            uni_ctrl.borrow_mut().cells.set(uni_ctrl.borrow().get_index(point.y, point.x), Cell::Dead.into());
                        },
                        CellControl::Toggle => {
                            uni_ctrl.borrow_mut().toggle_cell(point.y, point.x);
                        }
                    }
                }
                Some(x) = recv_p.recv() => {
                    match x {
                        PlayControl::Play => {
                            if let Some(ref mut p) = *p_ctrl.borrow_mut() {
                                cancel_animation_frame(*p).unwrap();
                            }
                            *p_ctrl.borrow_mut() =
                                Some(request_animation_frame(cls_ctrl.borrow().as_ref().unwrap()).unwrap());
                        }
                        PlayControl::Pause => {
                            if let Some(ref mut p) = *p_ctrl.borrow_mut() {
                                cancel_animation_frame(*p).unwrap();
                            }
                            *p_ctrl.borrow_mut() = None;
                        }
                    }
                }

            }
        }
    });

    // アニメーション開始と再生継続と停止のためのコールバック
    let p_closure = p.clone();
    let closure_clone = closure.clone();
    *closure_clone.borrow_mut() = Some(Closure::<
        dyn FnMut(f64) -> std::result::Result<i32, JsValue>,
    >::new(move |_time| {
        uni.borrow_mut().tick();
        drawer.draw_cells(&context, &uni.borrow());
        drawer.draw_grid(&context);
        fps.render();
        let res = request_animation_frame(closure.borrow().as_ref().unwrap());
        match res {
            Ok(handle) => {
                *p_closure.borrow_mut() = Some(handle);
                Ok(handle)
            }
            Err(e) => Err(e),
        }
    }));
    *p.borrow_mut() =
        Some(request_animation_frame(closure_clone.borrow().as_ref().unwrap()).unwrap());

    play_button_start(play_btn, sender);
    Ok(())
}

// 次のアニメーションフレームをリクエストする
fn request_animation_frame(
    closure: &Closure<dyn FnMut(f64) -> std::result::Result<i32, JsValue>>,
) -> std::result::Result<i32, JsValue> {
    let window = web_sys::window().unwrap();
    // JsValueが帰ってきておりJsErrorに変換できないのでunwrapしている
    window.request_animation_frame(closure.as_ref().unchecked_ref())
}

// 再生リクエストをキャンセル
fn cancel_animation_frame(handle: i32) -> std::result::Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    window.cancel_animation_frame(handle)
}

// [CellControl]とともに送信して、書き換えるセルの位置を指示
#[derive(Debug)]
struct Point {
    x: u32,
    y: u32,
}

// セルの状態変更指示
// enumはC-Styleのみサポート
#[derive(Debug)]
enum CellControl {
    Alive,
    Dead,
    Toggle,
}

/// 再生停止指示
#[wasm_bindgen]
#[derive(Debug)]
pub enum PlayControl {
    Play,
    Pause,
}

// JSからの指示を受け取るための構造体
#[wasm_bindgen]
pub struct Sender {
    p_ctrl: mpsc::UnboundedSender<PlayControl>,
    c_ctrl: mpsc::UnboundedSender<(CellControl, Point)>,
}

/// JSからのWasmに指示を飛ばすための構造体
#[wasm_bindgen]
impl Sender {
    fn new() -> (
        Self,
        mpsc::UnboundedReceiver<PlayControl>,
        mpsc::UnboundedReceiver<(CellControl, Point)>,
    ) {
        let (p_ctrl, recv_p) = mpsc::unbounded_channel();
        let (c_ctrl, recv_c) = mpsc::unbounded_channel();
        (Sender { p_ctrl, c_ctrl }, recv_p, recv_c)
    }

    pub fn play(&self, ctrl: PlayControl) {
        self.p_ctrl.send(ctrl).unwrap();
    }
}

// CanbasContext2Dで描画する実装
struct Drawer {
    alive_color: &'static str,
    dead_color: &'static str,
    cell_size: f64,
}

impl Drawer {
    fn draw_cells(&self, ctx: &CanvasRenderingContext2d, uni: &Universe) {
        let cell_size = self.cell_size;
        ctx.set_fill_style(&self.alive_color.into());

        ctx.begin_path();

        for row in 0..uni.height {
            for col in 0..uni.width {
                let idx = uni.get_index(row, col);
                let cell = uni.cells[idx];
                if cell == Cell::Alive.bool() {
                    ctx.fill_rect(
                        col as f64 * (cell_size + 1.0) + 1.0,
                        row as f64 * (cell_size + 1.0) + 1.0,
                        cell_size,
                        cell_size,
                    );
                }
            }
        }

        ctx.set_fill_style(&self.dead_color.into());

        for row in 0..uni.height {
            for col in 0..uni.width {
                let idx = uni.get_index(row, col);
                let cell = uni.cells[idx];
                if cell == Cell::Dead.bool() {
                    ctx.fill_rect(
                        col as f64 * (cell_size + 1.0) + 1.0,
                        row as f64 * (cell_size + 1.0) + 1.0,
                        cell_size,
                        cell_size,
                    );
                }
            }
        }

        ctx.stroke();
    }

    fn draw_grid(&self, ctx: &CanvasRenderingContext2d) {
        ctx.begin_path();
        ctx.set_stroke_style(&GRID_COLOR.into());

        let cs = self.cell_size + 1.0;

        // Vertical lines.
        for i in 0..ctx.canvas().unwrap().width() {
            ctx.move_to(i as f64 * cs + 1.0, 0.0);
            ctx.line_to(i as f64 * cs + 1.0, ctx.canvas().unwrap().height() as f64);
        }

        // Horizontal lines.
        for j in 0..ctx.canvas().unwrap().height() {
            ctx.move_to(0.0, j as f64 * cs + 1.0);
            ctx.line_to(ctx.canvas().unwrap().width() as f64, j as f64 * cs + 1.0);
        }

        ctx.stroke();
    }
}

impl Default for Drawer {
    fn default() -> Self {
        Drawer {
            alive_color: "#000000",
            dead_color: "#FFFFFF",
            cell_size: 5.0,
        }
    }
}

#[wasm_bindgen]
pub fn webgl_start(canvas: HtmlCanvasElement) -> Result<()> {
    use crate::webgl::basic_plane::*;
    canvas.set_width(256);
    canvas.set_height(256);

    let gl = canvas
        .get_context("webgl2")
        .map_err(|_| JsError::new("failed to get webgl2 context"))?
        .ok_or(JsError::new("Failed to get WebGl2RenderingContext"))?
        .dyn_into::<gl>()
        .map_err(|_| JsError::new("failed to cast to WebGl2RenderingContext"))?;

    let shader = Shader::new(&gl)?;
    let camera = Camera::default();
    let view = ViewMatrix::default();

    gl.enable(gl::DEPTH_TEST);
    gl.depth_func(gl::LEQUAL);
    gl.enable(gl::CULL_FACE);

    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear_depth(1.0);
    gl.clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

    shader.use_program(&gl);
    shader.set_mvp(&gl, &camera, &view);
    shader.draw(&gl);

    Ok(())
}

fn play_button_start(btn: web_sys::HtmlButtonElement, sender: Sender) {
    let sender = Rc::new(RefCell::new(sender));
    let ctx = Rc::new(RefCell::new(btn));
    let is_paused = Rc::new(RefCell::new(true));
    let is_paused_clone = Rc::clone(&is_paused);
    let sender_clone = sender.clone();
    let ctx_clone = ctx.clone();
    let closure = Closure::wrap(Box::new(move || {
        let is_paused = *is_paused_clone.borrow();
        if is_paused {
            sender_clone.borrow().play(PlayControl::Play);
            ctx_clone.borrow().set_text_content(Some("⏸"));
        } else {
            sender_clone.borrow().play(PlayControl::Pause);
            ctx_clone.borrow().set_text_content(Some("▶"));
        }
        *is_paused_clone.borrow_mut() = !is_paused;
    }) as Box<dyn FnMut()>);

    ctx.borrow()
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    // start play
    sender.borrow().play(PlayControl::Play);
    ctx.borrow().set_text_content(Some("⏸"));
}

struct Fps {
    element: web_sys::HtmlElement,
    performance: web_sys::Performance,
    frames: Vec<f64>,
    last_ts: f64,
}

impl Fps {
    fn new(fps: web_sys::HtmlElement) -> Self {
        let performance = web_sys::window().unwrap().performance().unwrap();
        Fps {
            element: fps,
            performance,
            frames: Vec::new(),
            last_ts: 0.0,
        }
    }
    fn render(&mut self) {
        let now = self.performance.now();
        let delta = now - self.last_ts;
        self.last_ts = now;
        let fps = 1000.0 / delta;
        self.frames.push(fps);
        if self.frames.len() > 60 {
            self.frames.remove(0);
        }
        let avg = self.frames.iter().sum::<f64>() / self.frames.len() as f64;
        let min = self.frames.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = self
            .frames
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        self.element.set_inner_text(&format!(
            r#"Frames per Second:
           latest = {fps:.3}
  avg of last 100 = {avg:.3}
  min of last 100 = {min:.3}
  max of last 100 = {max:.3}"#
        ));
    }
}

/// WASMのエントリポイント
/// JSから関数を呼ばなくても実行される
#[wasm_bindgen(start)]
pub fn run() -> Result<()> {
    log!("Hello, wasm-bindgen!");

    // 非同期ループ実験
    let token = tokio_util::sync::CancellationToken::new();
    let token_clone = token.clone();
    // 無限ループと条件付き終了
    // tokio spawnと違って戻り地がないため結果確認はできない
    wasm_bindgen_futures::spawn_local(async move {
        // 実行スレッドは1つしか無いのでawaitがなければ画面は固まる
        // 確認は Google Chrome 125.0.6422.60 at 2024/07/12
        loop {
            tokio::select! {
                _ = token_clone.cancelled() => {
                    log!("cancelled");
                    break;
                }
                // interval実装は無いため都度newする
                _ = TimeoutFuture::new(1_000) => {
                    log!("tick1");
                }
            }
        }
        log!("ticker finished");
    });

    // 上のFuture loopを停止するFuture
    wasm_bindgen_futures::spawn_local(async move {
        match fetch_example::<Hello>("/api/hello").await {
            Ok(val) => {
                log!("fetch_example: {:?}", val);
            }
            Err(e) => {
                jserror(e);
            }
        };
        TimeoutFuture::new(4_000).await;
        token.cancel();
    });

    start_websocket("ws://localhost:8080/api/ws/echo")?;
    Ok(())
}

async fn fetch_example<T: serde::de::DeserializeOwned>(url: &str) -> Result<T> {
    // fetch apiをラップしているgoo-netを使ってリクエストを送る
    let res = Request::get(url).send().await?;
    Ok(res.json::<T>().await?)
}

#[derive(Debug, serde::Deserialize)]
struct Hello {
    msg: String,
}

// websocketのタスクを開始する
fn start_websocket(url: &str) -> Result<()> {
    use futures::{SinkExt, StreamExt};
    let ws = WebSocket::open(url).map_err(gloo_net::Error::JsError)?;

    let (mut write, mut read) = ws.split();

    wasm_bindgen_futures::spawn_local(async move {
        let mut count = 0;
        loop {
            write
                .send(Message::Text(format!("test {}", count)))
                .await
                .unwrap();
            count += 1;
            TimeoutFuture::new(1_000).await;
        }
    });

    wasm_bindgen_futures::spawn_local(async move {
        while let Some(msg) = read.next().await {
            log!("1. {:?}", msg);
        }
        log!("WebSocket Closed");
    });
    Ok(())
}

#[wasm_bindgen]
pub fn webgl_interaction(
    canvas: HtmlCanvasElement,
    ctrl: ParticleControl,
) -> std::result::Result<(), JsValue> {
    use crate::webgl::interaction::*;
    canvas.set_width(512);
    canvas.set_height(512);

    let gl = canvas
        .get_context("webgl2")
        .map_err(|_| JsError::new("failed to get webgl2 context"))?
        .ok_or(JsError::new("Failed to get WebGl2RenderingContext"))?
        .dyn_into::<gl>()
        .map_err(|_| JsError::new("failed to cast to WebGl2RenderingContext"))?;
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear(gl::COLOR_BUFFER_BIT);

    let res = Resolution::DEFAULT;
    let mut shader = ParticleShader::new(&gl, res, ctrl)?;
    gl.enable(gl::BLEND);
    gl.blend_func_separate(gl::SRC_ALPHA, gl::ONE, gl::ONE, gl::ONE);

    // mouse event
    let canvas_ctx = Rc::new(RefCell::new(canvas));
    let (hander, mut recv) = MouseEventHandler::new(canvas_ctx.clone());
    hander.start();

    // アニメーションループ
    let mouse_pos = Rc::new(RefCell::new(Point::new(0., 0.)));
    let mouse_down_flag = Rc::new(RefCell::new(false));
    let mut a = AnimationLoop::new(move |timestamp_msec| {
        let t = timestamp_msec as f32;
        let color = hsva(t / 30., 1.0, 1.0, 0.5);
        gl.clear(gl::COLOR_BUFFER_BIT);
        shader.set_color(&gl, color);

        let event = {
            let mut event = None;
            while let Ok(e) = recv.try_recv() {
                event = Some(e);
            }
            event
        };
        match event {
            Some(MouseMessage::Move(pos)) => {
                *mouse_pos.borrow_mut() = pos;
                *mouse_down_flag.borrow_mut() = true;
            }
            Some(MouseMessage::Off) => {
                *mouse_down_flag.borrow_mut() = false;
            }
            None => {}
        }
        shader.update(&gl, *mouse_pos.borrow(), *mouse_down_flag.borrow());
        shader.draw(&gl);
        Ok(())
    });
    a.start();
    a.forget();

    Ok(())
}

#[derive(Debug)]
enum MouseMessage {
    Move(crate::webgl::interaction::Point),
    Off,
}

struct MouseEventHandler {
    canvas: Rc<RefCell<web_sys::HtmlCanvasElement>>,
    sender: UnboundedSender<MouseMessage>,
}

impl MouseEventHandler {
    fn new(
        canvas: Rc<RefCell<web_sys::HtmlCanvasElement>>,
    ) -> (Self, UnboundedReceiver<MouseMessage>) {
        let (sender, recv) = mpsc::unbounded_channel();
        let h = MouseEventHandler { canvas, sender };
        (h, recv)
    }

    fn get_point(
        canvas: &web_sys::HtmlCanvasElement,
        event: &web_sys::MouseEvent,
    ) -> crate::webgl::interaction::Point {
        use crate::webgl::interaction::Point;
        let pos = Point::new(event.client_x() as f32, event.client_y() as f32);
        let (offset_c, area_c) = {
            (
                Point::new(canvas.offset_left() as f32, canvas.offset_top() as f32),
                Point::new(canvas.width() as f32, canvas.height() as f32),
            )
        };
        let mut mouse_pos = (pos - offset_c - area_c / 2.) / area_c * 2.;
        mouse_pos.y = -mouse_pos.y;
        mouse_pos
    }

    fn start(self) {
        use crate::webgl::interaction::Point;

        let Self { canvas, sender } = self;

        let mouse_down_flag = Rc::new(RefCell::new(false));
        let mouse_down_flag_clone = mouse_down_flag.clone();
        let canvas_clone_down = canvas.clone();
        let sender_clone_down = sender.clone();
        let mouse_down = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            mouse_down_flag_clone.replace(true);
            let pos = Self::get_point(&canvas_clone_down.borrow(), &event);
            sender_clone_down.send(MouseMessage::Move(pos)).unwrap();
            log!("mouse down");
        }) as Box<dyn FnMut(_)>);

        let mouse_down_flag_clone = mouse_down_flag.clone();
        let sender_clone_up = sender.clone();
        let mouse_up = Closure::wrap(Box::new(move |_: web_sys::MouseEvent| {
            mouse_down_flag_clone.replace(false);
            sender_clone_up.send(MouseMessage::Off).unwrap();
        }) as Box<dyn FnMut(_)>);
        let mouse_down_flag_clone = mouse_down_flag.clone();

        let canvas_clone = canvas.clone();
        let mouse_pos = Rc::new(RefCell::new(Point::new(0., 0.)));
        let mouse_pos_clone = mouse_pos.clone();
        let mouse_move = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            if *mouse_down_flag_clone.borrow() {
                let pos = Self::get_point(&canvas_clone.borrow(), &event);
                sender.send(MouseMessage::Move(pos)).unwrap();
                mouse_pos_clone.replace(pos);
            }
        }) as Box<dyn FnMut(_)>);
        canvas
            .borrow()
            .add_event_listener_with_callback("mousedown", mouse_down.as_ref().unchecked_ref())
            .unwrap();
        mouse_down.forget();
        canvas
            .borrow()
            .add_event_listener_with_callback("mouseup", mouse_up.as_ref().unchecked_ref())
            .unwrap();
        mouse_up.forget();
        canvas
            .borrow()
            .add_event_listener_with_callback("mousemove", mouse_move.as_ref().unchecked_ref())
            .unwrap();
        mouse_move.forget();
    }
}

#[wasm_bindgen]
pub fn webgl_interaction_gpgpu(canvas: HtmlCanvasElement, ctrl: ParticleControl) -> Result<()> {
    use crate::webgl::interaction::*;
    canvas.set_width(512);
    canvas.set_height(512);
    let target_res = Resolution::new(512, 512);

    let gl = canvas
        .get_context("webgl2")
        .map_err(|_| JsError::new("failed to get webgl2 context"))?
        .ok_or(JsError::new("Failed to get WebGl2RenderingContext"))?
        .dyn_into::<gl>()
        .map_err(|_| JsError::new("failed to cast to WebGl2RenderingContext"))?;
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear(gl::COLOR_BUFFER_BIT);

    log!("extensions: {:?}", gl.get_supported_extensions());

    let unit_count = gl
        .get_parameter(gl::MAX_VERTEX_TEXTURE_IMAGE_UNITS)
        .unwrap()
        .as_f64()
        .unwrap() as u32;
    if unit_count < 1 {
        Err(JsError::new(
            "MAX_VERTEX_TEXTURE_IMAGE_UNITS is less than 1",
        ))?;
    }
    log!("MAX_VERTEX_TEXTURE_IMAGE_UNITS: {:?}", unit_count);

    // 浮動小数点数テクスチャが利用可能かどうかチェック
    // WebGL2なら常に利用可能なはず
    if gl
        .get_extension("EXT_color_buffer_float")
        .unwrap()
        .is_none()
    {
        Err(JsError::new("EXT_color_buffer_float is not supported"))?;
    }

    let mut shader = ParticleGpgpuShader::new(&gl, target_res, ctrl)?;
    gl.enable(gl::BLEND);
    gl.blend_func_separate(gl::SRC_ALPHA, gl::ONE, gl::ONE, gl::ONE);

    // test rendering
    shader.update(&gl, Point::new(0., 0.), true, [1.0, 0.0, 0.0, 1.0]);
    // shader.draw_index(&gl, &target_res);
    shader.draw(&gl, &target_res);

    // mouse event
    let canvas_ctx = Rc::new(RefCell::new(canvas));
    let (hander, mut recv) = MouseEventHandler::new(canvas_ctx.clone());
    hander.start();

    let mouse_pos = Rc::new(RefCell::new(Point::new(0., 0.)));
    let mouse_down_flag = Rc::new(RefCell::new(false));
    let mut a = AnimationLoop::new(move |timestamp_msec| {
        let t = timestamp_msec as f32;
        let color = hsva(t / 30., 1.0, 1.0, 0.5);

        let event = {
            let mut event = None;
            while let Ok(e) = recv.try_recv() {
                event = Some(e);
            }
            event
        };

        match event {
            Some(MouseMessage::Move(pos)) => {
                *mouse_pos.borrow_mut() = pos;
                *mouse_down_flag.borrow_mut() = true;
            }
            Some(MouseMessage::Off) => {
                *mouse_down_flag.borrow_mut() = false;
            }
            None => {}
        }

        shader.update(&gl, *mouse_pos.borrow(), *mouse_down_flag.borrow(), color);
        shader.draw(&gl, &target_res);
        Ok(())
    });

    a.start();
    a.forget();

    Ok(())
}

fn hsva(h: f32, s: f32, v: f32, a: f32) -> [f32; 4] {
    if s > 1. || v > 1. || a > 1. {
        return [1., 1., 1., 1.];
    }
    let th = h % 360.;
    let i = (th / 60.).floor();
    let f = th / 60. - i;
    let m = v * (1. - s);
    let n = v * (1. - s * f);
    let k = v * (1. - s * (1. - f));
    if s == 0. {
        [v, v, v, a]
    } else {
        let i = i as usize;
        let r = [v, n, m, m, k, v];
        let g = [k, v, v, n, m, m];
        let b = [m, m, k, v, v, n];
        [r[i], g[i], b[i], a]
    }
}
