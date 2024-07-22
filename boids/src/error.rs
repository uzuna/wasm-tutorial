use wasm_bindgen::JsValue;

pub type Result<T> = std::result::Result<T, Error>;

/// AsssetControlクレートのエラー型
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("WegGL Error {0}")]
    Gl(#[from] webgl2::error::Error),
    #[error(transparent)]
    GlooNet(#[from] gloo_net::Error),
}

impl Error {
    pub fn gl(msg: String) -> Self {
        webgl2::error::Error::gl(msg).into()
    }
}

impl From<Error> for JsValue {
    fn from(e: Error) -> Self {
        JsValue::from_str(&format!("{:?}", e))
    }
}
