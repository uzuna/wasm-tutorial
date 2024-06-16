mod utils;

use std::{
    cell::{self, RefCell},
    fmt,
    rc::Rc,
};

use fixedbitset::FixedBitSet;
use js_sys::Math::random;
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

    pub fn build(&self) -> Universe {
        // set canvas size
        self.canvas.set_width((self.width + 1) * self.cell_size);
        self.canvas.set_height((self.height + 1) * self.cell_size);
        Universe::new(self.width, self.height)
    }

    // event callbackチェエク
    pub fn gol(self) {
        let ue = UniEventer {
            cell_size: self.cell_size,
            canvas: self.canvas,
        };

        let ctx = Rc::new(RefCell::new(ue));

        let ctx_clone = Rc::clone(&ctx);
        let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
            let x = event.offset_x() as u32 / ctx_clone.borrow().cell_size;
            let y = event.offset_y() as u32 / ctx_clone.borrow().cell_size;
            log!("click: ({}, {})", x, y);
        }) as Box<dyn FnMut(_)>);
        ctx.borrow()
            .canvas
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();

        // closureはevent_listenerに渡したので、dropさせない
        closure.forget();
    }
}

#[wasm_bindgen]
pub struct UniEventer {
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
