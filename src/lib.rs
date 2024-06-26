mod utils;

use fixedbitset::FixedBitSet;
use futures_util::stream::StreamExt;
use js_sys::Math::random;
use std::{cell::RefCell, fmt, rc::Rc};
use tokio::sync::mpsc::{self, UnboundedSender};
use wasm_bindgen::prelude::*;

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

/// ライフゲームのビルダー
/// 複雑な引数を渡すテスト
#[wasm_bindgen]
pub struct GolBuilder {
    width: u32,
    height: u32,
    cell_size: u32,
    canvas: web_sys::HtmlCanvasElement,
}

/// 関数をこう飽きする場合はimplにwasm_bindgenをつけてpubにする
#[wasm_bindgen]
impl GolBuilder {
    pub fn new(width: u32, height: u32, canvas: web_sys::HtmlCanvasElement) -> GolBuilder {
        GolBuilder {
            width,
            height,
            cell_size: 5,
            canvas,
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
                let symbol = if cell == Cell::Dead.into() {
                    '◻'
                } else {
                    '◼'
                };
                write!(f, "{}", symbol)?;
            }
            write!(f, "\n")?;
        }

        Ok(())
    }
}

impl Universe {
    /// 幅を設定する。(u32, u32)はWASMの制約により使えないのでwasm_bindgenを使わない
    pub fn set_cells(&mut self, cells: &[(u32, u32)]) {
        for (row, col) in cells.iter().cloned() {
            let idx = self.get_index(row, col);
            self.cells.set(idx, Cell::Alive.into());
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
pub fn golstart(gb: GolBuilder) -> Result<Sender, JsValue> {
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
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    gb.gol(sender.c_ctrl.clone());

    // 非同期タイマー実験
    wasm_bindgen_futures::spawn_local(async move {
        let ticker = gloo_timers::future::IntervalStream::new(2000);
        ticker
            .for_each(|_| async {
                log!("tick");
            })
            .await;
    });

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
    *closure_clone.borrow_mut() = Some(Closure::<dyn FnMut() -> Result<i32, JsValue>>::new(
        move || {
            uni.borrow_mut().tick();
            drawer.draw_cells(&context, &uni.borrow());
            let res = request_animation_frame(closure.borrow().as_ref().unwrap());
            match res {
                Ok(handle) => {
                    *p_closure.borrow_mut() = Some(handle);
                    Ok(handle)
                }
                Err(e) => Err(e),
            }
        },
    ));
    *p.borrow_mut() = Some(request_animation_frame(
        closure_clone.borrow().as_ref().unwrap(),
    )?);
    Ok(sender)
}

// 次のアニメーションフレームをリクエストする
fn request_animation_frame(
    closure: &Closure<dyn FnMut() -> Result<i32, JsValue>>,
) -> Result<i32, JsValue> {
    let window = web_sys::window().unwrap();
    window.request_animation_frame(closure.as_ref().unchecked_ref())
}

// 再生リクエストをキャンセル
fn cancel_animation_frame(handle: i32) -> Result<(), JsValue> {
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
    fn draw_cells(&self, ctx: &web_sys::CanvasRenderingContext2d, uni: &Universe) {
        let cell_size = self.cell_size;
        ctx.set_fill_style(&self.alive_color.into());

        ctx.begin_path();

        for row in 0..uni.height {
            for col in 0..uni.width {
                let idx = uni.get_index(row, col);
                let cell = uni.cells[idx];
                if cell == Cell::Alive.into() {
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
                if cell == Cell::Dead.into() {
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
