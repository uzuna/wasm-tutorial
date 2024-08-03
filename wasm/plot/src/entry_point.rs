use std::time::Duration;

use nalgebra::Vector2;
use rand::Rng;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use webgl2::GlPoint2d;

use crate::shader::PlotParams;

#[wasm_bindgen(start)]
pub fn init() -> Result<(), JsValue> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(canvas: HtmlCanvasElement) -> Result<(), JsValue> {
    canvas.set_width(1024);
    canvas.set_height(768);

    let gl = webgl2::context::get_webgl2_context(&canvas, webgl2::context::COLOR_BLACK)?;

    // 1プロットグラフのパラメータ
    let mut prop = PlotParams::default();
    prop.point_size = 12.0;
    let mut shader = crate::shader::PlotShader::new(&gl, &prop)?;
    let mut rx = walker(RandomWalk::new(), Duration::from_millis(50));

    let a = wasm_utils::animation::AnimationLoop::new(move |time| {
        // 秒単位に調整
        let time = (time / 1000.0) as f32;

        // データを受信。shaderと組にする
        while let Ok(x) = rx.try_recv() {
            let p = GlPoint2d::new(time, x);
            shader.add_data(&gl, p);
        }

        // 画面いっぱいにプロットするための調整
        let time_scale = 0.5; // xは2.0の長さなのでプロットの生存期間を4secにするために0.5
        let start_offset = 1.0 / time_scale; // 幅が広くなった分データの開始位置が右橋になるように調整

        let mat = nalgebra::Matrix3::identity()
            .append_translation(&Vector2::new(-time + start_offset, 0.0))
            .append_nonuniform_scaling(&Vector2::new(time_scale, 0.1));

        // matは各シェーダーに共通してもいいし、異なるレンジで描くなら分けても良い
        // shaderごとにViewPortを調整するのが良さそう
        shader.set_window_mat(&gl, mat);

        webgl2::context::gl_clear_color(&gl, webgl2::context::COLOR_BLACK);
        shader.draw(&gl);
        Ok(())
    });
    a.start();
    a.forget();

    Ok(())
}

struct RandomWalk {
    x: f32,
    rng: rand::rngs::ThreadRng,
    range: std::ops::RangeInclusive<f32>,
}

impl RandomWalk {
    fn new() -> Self {
        Self {
            x: 0.0,
            rng: rand::thread_rng(),
            range: -0.1..=0.1,
        }
    }

    fn next(&mut self) -> f32 {
        self.x += self.rng.gen_range(self.range.clone());
        self.x
    }
}

fn walker(mut w: RandomWalk, interval: Duration) -> UnboundedReceiver<f32> {
    use futures_util::{future::ready, stream::StreamExt};
    let (tx, rx) = unbounded_channel();
    wasm_bindgen_futures::spawn_local(async move {
        gloo_timers::future::IntervalStream::new(interval.as_millis() as u32)
            .for_each(|_| {
                tx.send(w.next()).unwrap();
                ready(())
            })
            .await;
    });

    rx
}
