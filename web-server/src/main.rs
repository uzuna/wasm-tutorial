use std::net::SocketAddr;

use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use hex_color::HexColor;
use image::{ImageBuffer, ImageEncoder, Rgba};
use rand::Rng;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_static_file_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let serve_dir = ServeDir::new("assets").append_index_html_on_directories(true);
    let router = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/hello", get(Hello::get_response))
                .route("/ws/echo", get(echo_ws))
                .route("/ws/boid/gen_stream", get(gen_boid_ws))
                .route("/texture/generate/:name", get(gen_texture))
                .route("/sleep/:msec", get(get_sleep)),
        )
        .fallback_service(serve_dir)
        .layer(TraceLayer::new_for_http());

    let port = 8080;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, router).await.unwrap();
}

/// A simple JSON response
#[derive(Debug, serde::Serialize)]
struct Hello {
    msg: &'static str,
}

impl Hello {
    const DEFAULT: Self = Self {
        msg: "hello world!",
    };
    async fn get_response() -> impl IntoResponse {
        Json(Self::DEFAULT)
    }
}

async fn echo_ws(ws: axum::extract::ws::WebSocketUpgrade) -> impl IntoResponse {
    use futures_util::{stream::StreamExt, SinkExt};
    ws.on_upgrade(|socket| async {
        let (mut sender, mut receiver) = socket.split();
        while let Some(msg) = receiver.next().await {
            let msg = msg.unwrap();
            sender.send(msg).await.unwrap();
        }
    })
}

#[derive(Debug, serde::Serialize)]
struct CreateBoidRequest {
    pos: [f32; 3],
    vel: [f32; 3],
}

impl CreateBoidRequest {
    fn rand() -> Self {
        let mut rnd = rand::thread_rng();
        Self {
            pos: [rnd.gen(), rnd.gen(), rnd.gen()],
            vel: [rnd.gen(), rnd.gen(), rnd.gen()],
        }
    }
}

/// boidを生成するリクエストを投げ続ける
async fn gen_boid_ws(ws: axum::extract::ws::WebSocketUpgrade) -> impl IntoResponse {
    use futures_util::{stream::StreamExt, SinkExt};
    ws.on_upgrade(|socket| async {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(5));
        let (mut sender, _receiver) = socket.split();
        loop {
            let mut buf = Vec::new();
            let req = CreateBoidRequest::rand();
            ciborium::into_writer(&req, &mut buf).unwrap();
            sender
                .send(axum::extract::ws::Message::Binary(buf))
                .await
                .unwrap();
            ticker.tick().await;
        }
    })
}

/// 画像フォーマット
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
enum ImageFormat {
    Qoi,
    #[default]
    Png,
    Jpeg,
    Webp,
}

/// 画像生成リクエスト
#[derive(Debug, Default, PartialEq, serde::Deserialize)]
struct TextureQuery {
    width: Option<u32>,
    height: Option<u32>,
    format: Option<ImageFormat>,
    color_front: Option<String>,
    color_back: Option<String>,
}

impl TextureQuery {
    fn width(&self) -> u32 {
        self.width.unwrap_or(128)
    }
    fn height(&self) -> u32 {
        self.height.unwrap_or(128)
    }
    fn format(&self) -> ImageFormat {
        self.format.unwrap_or_default()
    }
    fn color_front(&self) -> [u8; 4] {
        Self::parse_color(self.color_front.as_deref(), [128, 128, 128, 255])
    }
    fn color_back(&self) -> [u8; 4] {
        Self::parse_color(self.color_back.as_deref(), [0, 0, 0, 255])
    }
    fn parse_color(color: Option<&str>, default: [u8; 4]) -> [u8; 4] {
        match color {
            Some(color) => match HexColor::parse(color) {
                Ok(color) => [color.r, color.g, color.b, color.a],
                Err(e) => {
                    tracing::warn!("failed to parse color: {:?}", e);
                    default
                }
            },
            None => default,
        }
    }
}

fn write_image(
    img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    format: ImageFormat,
) -> Result<Vec<u8>, image::error::ImageError> {
    use image::ExtendedColorType::Rgba8;
    let mut buf = Vec::new();
    match format {
        ImageFormat::Qoi => {
            use image::codecs::qoi::QoiEncoder;
            let encoder = QoiEncoder::new(&mut buf);
            encoder.write_image(img, img.width(), img.height(), Rgba8)?
        }
        ImageFormat::Png => {
            use image::codecs::png::{CompressionType::Best, FilterType::NoFilter, PngEncoder};
            let encoder = PngEncoder::new_with_quality(&mut buf, Best, NoFilter);
            encoder.write_image(img, img.width(), img.height(), Rgba8)?;
        }
        ImageFormat::Jpeg => {
            use image::codecs::jpeg::JpegEncoder;
            let encoder = JpegEncoder::new_with_quality(&mut buf, 100);
            encoder.write_image(img, img.width(), img.height(), Rgba8)?;
        }
        ImageFormat::Webp => {
            use image::codecs::webp::WebPEncoder;
            let encoder = WebPEncoder::new_lossless(&mut buf);
            encoder.write_image(img, img.width(), img.height(), Rgba8)?;
        }
    }
    Ok(buf)
}

async fn gen_texture(
    axum::extract::Path(_name): axum::extract::Path<String>,
    query: axum::extract::Query<TextureQuery>,
) -> impl IntoResponse {
    use image::{ImageBuffer, Rgba};

    // parse query
    let front_color = Rgba(query.color_front());
    let back_color = Rgba(query.color_back());
    let width = query.width();
    let height = query.height();
    let format = query.format();

    // generage image
    let img = ImageBuffer::from_fn(width, height, |x, y| match (x, y) {
        (x, y) if x < width / 2 && y < height / 2 => front_color,
        (x, y) if x >= width / 2 && y >= height / 2 => front_color,
        _ => back_color,
    });

    match write_image(&img, format) {
        Ok(buf) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "image/png")],
            buf,
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(axum::http::header::CONTENT_TYPE, "text/plain")],
            format!("failed to generate image: {:?}", e)
                .to_owned()
                .into_bytes(),
        ),
    }
}

async fn get_sleep(axum::extract::Path(msec): axum::extract::Path<u64>) -> impl IntoResponse {
    tokio::time::sleep(std::time::Duration::from_millis(msec)).await;
    format!("slept {msec} msec").into_response()
}
