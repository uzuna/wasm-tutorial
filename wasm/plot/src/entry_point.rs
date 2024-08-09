use std::{cell::RefCell, rc::Rc, sync::atomic::AtomicBool, time::Duration};

use nalgebra::Vector2;
use rand::Rng;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use wasm_bindgen::prelude::*;
use wasm_utils::{animation::PlayStopButton, error::*};
use web_sys::{HtmlButtonElement, HtmlCanvasElement};
use webgl2::{font::TextShader, font_asset::color_texture, gl};

use crate::{
    plot::{Chart, ViewPort},
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

    let aspect = 1024.0 / 768.0;
    let gl = webgl2::context::get_webgl2_context(&canvas, webgl2::context::COLOR_BLACK)?;
    let playing = Rc::new(RefCell::new(AtomicBool::new(false)));

    // 1Chart単位を手で組む
    let mut chart = Chart::new(ViewPort::new(0, 768 - 128, 1024, 128))?;
    let s1 = chart.add_series(
        &gl,
        PlotParams::new(Duration::from_secs(10), 30, (-10.0, 10.0)),
        "Random Walk 1",
    )?;

    let mut prop = PlotParams::new(Duration::from_secs(10), 10, (-5.0, 5.0));
    prop.color = [0.0, 1.0, 0.0, 1.0];
    let s2 = chart.add_series(&gl, prop, "Random Walk 2")?;

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
        &gl,
        ViewPort::new(0, 768 - 256, 1024, 128),
        prop.clone(),
        16,
        playing.clone(),
    )?;
    let (mut c3, mut dcm3) = random_walk_chart(
        &gl,
        ViewPort::new(0, 768 - 384, 1024, 128),
        prop.clone(),
        16,
        playing.clone(),
    )?;

    let font = webgl2::font_asset::load(&gl)?;
    let mut text = font.create_text_vertex("Hello,0000000000");
    let ts = TextShader::new(&gl)?;

    let mat = nalgebra::Matrix3::identity()
        .append_nonuniform_scaling(&Vector2::new(0.002, 0.002 * aspect));
    let mat: [[f32; 3]; 3] = mat.into();
    let mm = mat.iter().flat_map(|a| *a).collect::<Vec<_>>();
    ts.set_mat(&gl, &mm);
    let tv = ts.link_vertex(&gl, &text)?;
    gl.viewport(0, 0, 1024, 768);
    ts.draw(&gl, &tv);

    let a = wasm_utils::animation::AnimationLoop::new(move |time| {
        // データを受信。shaderと組にする
        dcm1.update(&gl, &mut chart);
        dcm2.update(&gl, &mut c2);
        dcm3.update(&gl, &mut c3);

        let current_time = time as f32 / 1000.0;
        webgl2::context::gl_clear_color(&gl, webgl2::context::COLOR_BLACK);
        chart.draw(&gl, current_time);
        c2.draw(&gl, current_time);
        c3.draw(&gl, current_time);
        // TODO 文字がプロットの下にレンダリングされる理由を特定する
        gl.viewport(0, 0, 1024, 768);
        font.update_text(&mut text, &format!("Hello,{}", time as u32));
        text.update_uv(&gl, &tv);
        ts.draw(&gl, &tv);
        Ok(())
    });

    // TODO: 止めるべきはAnimationLoopのインスタンスではなく、データ更新部分では?
    let btn = PlayStopButton::new(play_pause_btn, a, playing);

    let ctx = btn.start();
    // JSに戻したらGCで回収されたためforgetする
    ctx.forget();
    Ok(())
}

// 大量のデータを描画するテスト
fn random_walk_chart(
    gl: &gl,
    viewport: ViewPort,
    base_prop: PlotParams,
    series_count: u32,
    playing: Rc<RefCell<AtomicBool>>,
) -> Result<(Chart, DataChannelMap)> {
    let mut chart = Chart::new(viewport)?;
    for i in 0..series_count {
        let mut prop = base_prop.clone();
        let rgb = hsv_to_rgb(i as f64 * 360.0 / series_count as f64, 1.0, 1.0);
        prop.color = [rgb.0, rgb.1, rgb.2, 0.5];
        chart.add_series(gl, prop, &format!("Random Walk {}", i))?;
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

    fn update(&mut self, gl: &gl, chart: &mut Chart) {
        for (rx, index) in &mut self.v {
            while let Ok((time, value)) = rx.try_recv() {
                chart.add_data(gl, *index, time, value);
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
