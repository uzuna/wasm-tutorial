use std::time::Duration;

use rand::Rng;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{plot::Plot, shader::PlotParams};

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
    let prop = PlotParams::new(Duration::from_secs(10), 30);
    let mut p = Plot::new(&gl, prop)?;
    p.set_y_range((-10.0, 10.0));

    let mut prop = PlotParams::new(Duration::from_secs(10), 10);
    prop.color = [0.0, 1.0, 0.0, 1.0];
    let mut p2 = Plot::new(&gl, prop)?;
    p2.set_y_range((-5.0, 5.0));
    let mut rx = walker(RandomWalk::new(), Duration::from_millis(33));
    let mut rx2 = walker(RandomWalk::new(), Duration::from_millis(100));

    let mut a = wasm_utils::animation::AnimationLoop::new(move |time| {
        // データを受信。shaderと組にする
        while let Ok(x) = rx.try_recv() {
            p.add_data(&gl, x.0, x.1);
        }
        // データを受信。shaderと組にする
        while let Ok(x) = rx2.try_recv() {
            p2.add_data(&gl, x.0, x.1);
        }
        let current_time = time as f32 / 1000.0;
        p.update_window(&gl, current_time);
        p2.update_window(&gl, current_time);

        webgl2::context::gl_clear_color(&gl, webgl2::context::COLOR_BLACK);
        p.draw(&gl);
        p2.draw(&gl);
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

    fn next(&mut self) -> (f32, f32) {
        let time = web_sys::window()
            .expect("Failed to get window")
            .document()
            .expect("Failed to get performance")
            .timeline()
            .current_time()
            .expect("Failed to get current time")
            / 1000.;
        self.x += self.rng.gen_range(self.range.clone());
        (time as f32, self.x)
    }
}

fn walker(mut w: RandomWalk, interval: Duration) -> UnboundedReceiver<(f32, f32)> {
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
