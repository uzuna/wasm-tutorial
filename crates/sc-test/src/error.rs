pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Any error: {0}")]
    Any(#[from] anyhow::Error),
}
