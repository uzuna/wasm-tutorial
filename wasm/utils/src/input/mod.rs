use crate::error::*;

pub mod button;
pub mod select;
pub mod slider;
mod util;

/// HTML側のエレメントを探すための識別子を返す
pub trait InputIdent: Copy + 'static {
    fn id(&self) -> &'static str;
}

/// CheckBoxなどのBool型の場合に実装する
pub trait InputBool: Sized {
    fn value(&self) -> Result<bool>;
    fn with_value(&self, value: bool) -> Result<Self>;
}

/// f32型の場合に実装する
pub trait InputNumber<T>: Sized {
    fn value(&self) -> Result<T>;
    fn with_value(&self, value: T) -> Result<Self>;
}

/// SelectInputの場合に実装する
pub trait InputOption<O>: Sized
where
    O: SelectOption,
{
    fn value(&self) -> Result<O>;
    fn with_value(&self, value: O) -> Result<Self>;
}

pub trait SelectOption: Copy + Sized + 'static {
    fn iter() -> &'static [Self];
    fn value(&self) -> &str;
    fn text(&self) -> &str;
    fn from_str(value: &str) -> Self;
}
