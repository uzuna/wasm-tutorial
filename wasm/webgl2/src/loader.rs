use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use wasm_bindgen::prelude::*;
use web_sys::HtmlImageElement;

use crate::{error::*, texture::Texture};

/// 画像をHtmlImageElementを経由して読み込むFuture実装構造体
pub struct ImageLoader {
    // 読み込むためのエレメンt
    image: HtmlImageElement,
    // jsのコールバックを保持するための変数
    closure: Option<Closure<dyn FnMut()>>,
}

impl ImageLoader {
    pub fn new(path: impl AsRef<str>) -> Result<Self> {
        let image =
            HtmlImageElement::new().map_err(|_| JsError::new("failed to create image element"))?;
        image.set_src(path.as_ref());
        Ok(Self {
            image,
            closure: None,
        })
    }
}

impl Future for ImageLoader {
    type Output = Result<HtmlImageElement>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        // 非同期処理が呼ばれたら、画像を返すか、読み込みを待つ
        if self.image.complete() {
            Poll::Ready(Ok(self.image.clone()))
        } else {
            let waker = cx.waker().clone();
            let closure = Closure::wrap(Box::new(move || {
                waker.wake_by_ref();
            }) as Box<dyn FnMut()>);
            self.image
                .set_onload(Some(closure.as_ref().unchecked_ref()));
            self.closure = Some(closure);
            Poll::Pending
        }
    }
}

impl Drop for ImageLoader {
    fn drop(&mut self) {
        // 各リソースはdrop時に解放する
        self.image.set_onload(None);
        self.image.remove();
    }
}

// 非同期フローの中で画像の読み込みを待つ
pub async fn load_texture(src: impl AsRef<str>, texture: &Texture) -> Result<()> {
    let loader = ImageLoader::new(src)?;
    let img = loader.await?;
    texture.update_texture_image_element(&img);
    Ok(())
}
