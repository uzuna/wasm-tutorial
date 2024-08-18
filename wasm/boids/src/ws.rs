use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;

use wasm_utils::{error::*, info};

#[derive(serde::Deserialize)]
struct CreateBoidRequest {
    pos: [f32; 3],
    vel: [f32; 3],
}

// websocketのタスクを開始する
pub fn start_websocket(url: &str) -> Result<()> {
    use futures::StreamExt;
    let ws = WebSocket::open(url).map_err(gloo_net::Error::JsError)?;

    let (_write, mut read) = ws.split();

    wasm_bindgen_futures::spawn_local(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Bytes(byte)) => {
                    let x = ciborium::from_reader::<CreateBoidRequest, _>(byte.as_slice()).unwrap();
                    info!("byte pos: {:?}, vel: {:?}", x.pos, x.vel);
                }
                Ok(Message::Text(text)) => {
                    info!("text {:?}", text);
                }
                Err(e) => {
                    info!("error {:?}", e);
                }
            }
        }
        info!("WebSocket Closed");
    });
    Ok(())
}
