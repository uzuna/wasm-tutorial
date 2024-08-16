use nalgebra::Vector2;
use webgl2::gl;

/// Window表示インスタンスのうち、表示領域に使う領域を保持する
///
/// 単位はpx
pub struct ViewPort {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl ViewPort {
    /// canvasの表示範囲を指定。左上が原点
    pub fn new(x: i32, y: i32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    // px指定でOpenGLローカル座標を取得。左上原点
    pub fn local(&self, x: i32, y: i32, w: u32, h: u32) -> LocalView {
        // let scissor = Scissor::new(x, h as i32 - y, w as i32, h as i32);
        // y座標は下からなので反転
        let scissor = self.scissor_area(x, y, w, h);
        let ww = self.w as f32 / 2.0;
        let wh = self.h as f32 / 2.0;
        let x = (x as f32 - ww) / ww;
        let y = (wh - y as f32) / wh;
        let w = w as f32 / self.w as f32;
        let h = h as f32 / self.h as f32;
        LocalView {
            // 幅2.0のOpenGL空間に変換
            x: x + w,
            // Scissorと同じく左下原点のため、y反転させてh分下に移動
            y: y - h,
            w,
            h,
            aspect: self.aspect(),
            scissor,
        }
    }

    fn scissor_area(&self, x: i32, y: i32, w: u32, h: u32) -> Scissor {
        // scissorは左下原点なので、y座標を反転させてh幅分下に移動
        let y = self.h as i32 - y - h as i32;
        Scissor::new(x, y, w as i32, h as i32)
    }

    #[inline]
    pub fn aspect(&self) -> f32 {
        self.w as f32 / self.h as f32
    }

    pub fn scissor(&self, gl: &gl) {
        gl.enable(gl::SCISSOR_TEST);
        gl.scissor(self.x, self.y, self.w as i32, self.h as i32);
    }
}

/// レンダリング範囲をViewport内の一部に制限する
///
/// UI表示など、範囲外にレンダリングされてほしくない場合に使用
pub struct Scissor {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Scissor {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn scissor(&self, gl: &gl) {
        gl.scissor(self.x, self.y, self.w, self.h);
    }
}

/// 表示空間全体のうち、切り出された表示領域を保持する
///
/// 単位はOpenGL空間
pub struct LocalView {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    aspect: f32,
    scissor: Scissor,
}

impl LocalView {
    /// -1.0 -> 1.0の空間に変換
    pub fn local_mat(&self) -> nalgebra::Matrix3<f32> {
        // 計算順序に意味がある。スケール調整後に移動を行う。そうしなければ、移動量がスケールの影響を受ける
        nalgebra::Matrix3::identity()
            .append_nonuniform_scaling(&Vector2::new(self.w, self.h))
            .append_translation(&Vector2::new(self.x, self.y))
    }

    /// UIなどの場合は表示範囲外に表示をさせない
    pub fn scissor(&self, gl: &gl) {
        self.scissor.scissor(gl);
    }
}
