use tokio::sync::mpsc;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub trait Mergeable {
    fn merge(&mut self, other: Self);
}

// JSのイベントはアニメーションループと周期が異なるので複数のイベントが入っている場合がある
// フレーム更新時は最後の値を使う
pub fn merge_events<T: Mergeable>(rx: &mut mpsc::UnboundedReceiver<T>) -> Option<T> {
    let mut last: Option<T> = None;
    while let Ok(param) = rx.try_recv() {
        if let Some(ref mut last) = last {
            last.merge(param);
        } else {
            last = Some(param);
        }
    }
    last
}
