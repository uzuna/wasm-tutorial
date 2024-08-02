use tokio::sync::mpsc;
/// 異なるイベントからのパラメータをマージするためのトレイト
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
