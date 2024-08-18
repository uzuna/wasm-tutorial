use std::{cell::RefCell, rc::Rc, sync::atomic::AtomicBool, time::Duration};

use rand::Rng;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use wasm_bindgen::prelude::*;
use wasm_utils::{animation::PlayStopButton, error::*};
use web_sys::{HtmlButtonElement, HtmlCanvasElement};
use webgl2::{
    context::Context,
    font::{Align, TextShader},
    viewport::LocalView,
};

use crate::{
    plot::Chart,
    shader::{PlaneShader, PlotParams},
};

#[wasm_bindgen(start)]
pub fn init() -> Result<()> {
    wasm_utils::panic::set_panic_hook();
    Ok(())
}

#[wasm_bindgen]
pub fn start(
    canvas: HtmlCanvasElement,
    play_pause_btn: HtmlButtonElement,
) -> std::result::Result<(), JsValue> {
    canvas.set_width(1024);
    canvas.set_height(768);

    let ctx = webgl2::context::Context::new(canvas, webgl2::context::COLOR_BLACK)?;
    let viewport = ctx.viewport();
    let gl = ctx.gl().clone();
    webgl2::context::gl_clear_color(&gl, webgl2::context::COLOR_BLACK);

    let playing = Rc::new(RefCell::new(AtomicBool::new(false)));

    // 1Chart単位を手で組む
    let mut chart = Chart::new(&ctx, viewport.local(0, 0, 1024, 128))?;
    let s1 = chart.add_series(
        &ctx,
        PlotParams::new(Duration::from_secs(10), 30, (-10.0, 10.0)),
        "Random Walk 1",
    )?;

    let mut prop = PlotParams::new(Duration::from_secs(10), 10, (-5.0, 5.0));
    prop.color = [0.0, 1.0, 0.0, 1.0];
    let s2 = chart.add_series(&ctx, prop, "Random Walk 2")?;

    let mut dcm1 = DataChannelMap::new();
    dcm1.add(
        walker(
            RandomWalk::new(),
            Duration::from_millis(34),
            playing.clone(),
        ),
        s1,
    );
    dcm1.add(
        walker(
            RandomWalk::new(),
            Duration::from_millis(100),
            playing.clone(),
        ),
        s2,
    );

    let mut prop = PlotParams::new(Duration::from_secs(10), 100, (-5.0, 5.0));
    prop.point_size = 3.0;
    let (mut c2, mut dcm2) = random_walk_chart(
        &ctx,
        viewport.local(0, 128, 512, 128),
        prop.clone(),
        16,
        playing.clone(),
    )?;
    let (mut c3, mut dcm3) = random_walk_chart(
        &ctx,
        viewport.local(512, 256, 512, 128),
        prop.clone(),
        16,
        playing.clone(),
    )?;

    // フォント情報の読み出しとシェーダーの作成
    let font = webgl2::font_asset::load(&ctx)?;
    let ts = TextShader::new(&ctx)?;

    // テキストの頂点情報を作成し、VAOで描画メモリを確保
    let mut text = font.text_by_capacity(10, Align::left_bottom());
    let mat = viewport.font_mat(512, 128, 16.0);
    ts.local_mat(&mat);
    let tv = ts.create_vbo(&text)?;

    // ViewPort確認
    let lp = viewport.local(512, 256, 512, 128);
    let plane = PlaneShader::new(&ctx, [0.5, 0.5, 0.5, 1.0])?;
    plane.uniform().local_mat(lp.local_mat());
    plane.draw();

    // テキスト描画
    ts.draw(&tv);

    let a = wasm_utils::animation::AnimationLoop::new(move |time| {
        // データを受信。shaderと組にする
        dcm1.update(&mut chart);
        dcm2.update(&mut c2);
        dcm3.update(&mut c3);

        let current_time = time as f32 / 1000.0;
        webgl2::context::gl_clear_color(&gl, webgl2::context::COLOR_BLACK);
        chart.draw(current_time);
        c2.draw(current_time);
        c3.draw(current_time);

        // Scissorを解除して全体に描画
        viewport.scissor(&gl);
        plane.draw();

        // Chart.Seriese0の最後のデータを取得してテキストに反映
        if let Some(s) = chart.series(0) {
            if let Some((_time, value)) = s.last() {
                text.update_text(&format!("{:.5}", value));
                text.apply_to_vao(&tv);
                ts.draw(&tv);
            }
        }

        Ok(())
    });

    // TODO: 止めるべきはAnimationLoopのインスタンスではなく、データ更新部分では?
    let btn = PlayStopButton::new_with_flag(play_pause_btn, a, playing);

    let ctx = btn.start();
    // JSに戻したらGCで回収されたためforgetする
    ctx.forget();
    Ok(())
}

// 大量のデータを描画するテスト
fn random_walk_chart(
    ctx: &Context,
    localview: LocalView,
    base_prop: PlotParams,
    series_count: u32,
    playing: Rc<RefCell<AtomicBool>>,
) -> Result<(Chart, DataChannelMap)> {
    let mut chart = Chart::new(ctx, localview)?;
    for i in 0..series_count {
        let mut prop = base_prop.clone();
        let rgb = hsv_to_rgb(i as f64 * 360.0 / series_count as f64, 1.0, 1.0);
        prop.color = [rgb.0, rgb.1, rgb.2, 0.5];
        chart.add_series(ctx, prop, &format!("Random Walk {}", i))?;
    }

    let mut dcm = DataChannelMap::new();
    let pps = base_prop.point_per_seconds();
    let pps_duration = Duration::from_secs_f32(1.0 / pps);
    for i in 0..series_count {
        dcm.add(
            walker(RandomWalk::new(), pps_duration, playing.clone()),
            i as usize,
        );
    }
    Ok((chart, dcm))
}

// データチャンネルから受信してチャートのデータを更新するための関係性を保持する構造体
struct DataChannelMap {
    v: Vec<(UnboundedReceiver<(f32, f32)>, usize)>,
}

impl DataChannelMap {
    fn new() -> Self {
        Self { v: Vec::new() }
    }

    fn add(&mut self, rx: UnboundedReceiver<(f32, f32)>, index: usize) {
        self.v.push((rx, index));
    }

    fn update(&mut self, chart: &mut Chart) {
        for (rx, index) in &mut self.v {
            while let Ok((time, value)) = rx.try_recv() {
                chart.add_data(*index, time, value);
            }
        }
    }
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

fn walker(
    mut w: RandomWalk,
    interval: Duration,
    playing: Rc<RefCell<AtomicBool>>,
) -> UnboundedReceiver<(f32, f32)> {
    use futures_util::{future::ready, stream::StreamExt};
    let (tx, rx) = unbounded_channel();
    wasm_bindgen_futures::spawn_local(async move {
        gloo_timers::future::IntervalStream::new(interval.as_millis() as u32)
            .for_each(|_| {
                if playing
                    .borrow_mut()
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    let _ = tx.send(w.next());
                }
                ready(())
            })
            .await;
    });

    rx
}

// reference from https://github.com/jayber/hsv
pub fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> (f32, f32, f32) {
    fn is_between(value: f64, min: f64, max: f64) -> bool {
        min <= value && value < max
    }

    check_bounds(hue, saturation, value);

    let c = value * saturation;
    let h = hue / 60.0;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = value - c;

    let (r, g, b): (f64, f64, f64) = if is_between(h, 0.0, 1.0) {
        (c, x, 0.0)
    } else if is_between(h, 1.0, 2.0) {
        (x, c, 0.0)
    } else if is_between(h, 2.0, 3.0) {
        (0.0, c, x)
    } else if is_between(h, 3.0, 4.0) {
        (0.0, x, c)
    } else if is_between(h, 4.0, 5.0) {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    ((r + m) as f32, (g + m) as f32, (b + m) as f32)
}

fn check_bounds(hue: f64, saturation: f64, value: f64) {
    fn panic_bad_params(name: &str, from_value: &str, to_value: &str, supplied: f64) -> ! {
        panic!(
            "param {} must be between {} and {} inclusive; was: {}",
            name, from_value, to_value, supplied
        )
    }

    if !(0.0..=360.0).contains(&hue) {
        panic_bad_params("hue", "0.0", "360.0", hue)
    } else if !(0.0..=1.0).contains(&saturation) {
        panic_bad_params("saturation", "0.0", "1.0", saturation)
    } else if !(0.0..=1.0).contains(&value) {
        panic_bad_params("value", "0.0", "1.0", value)
    }
}
