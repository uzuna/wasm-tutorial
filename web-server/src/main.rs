use std::net::SocketAddr;

use axum::{response::IntoResponse, routing::get, Json, Router};
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
                .route("/ws/boid/gen_stream", get(gen_boid_ws)),
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
