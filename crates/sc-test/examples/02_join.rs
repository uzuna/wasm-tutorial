//! 01の表記を変更した
use sc_test::{signal, Actor, ActorIn, StWrapper, Target};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub fn main() -> anyhow::Result<()> {
    run_with_join()
}

// joinを使う場合
// こちらはblock無いで生存していることを必須としているので実行可能
fn run_with_join() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(run_with_join_inner())?;
    Ok(())
}

// [run_with_spawn]を書き換えたもの。
// こちらはjoinを使っているのでasync moveが不要
async fn run_with_join_inner() -> sc_test::error::Result<()> {
    let token = CancellationToken::new();
    let actor = Actor::new(0.0, 1.0);
    let mut actor_stw = StWrapper::new(actor);
    let actor_tx: mpsc::Sender<ActorIn> = actor_stw.tx();
    let mut target = Target::new(10.0, 1.0);

    // この思索の主題。静的な同時実行とは、スケジューリングが同時であれば良くて、並行実行(CPUコア別で実行される)必要とは別の要件
    // 言葉の定義は[タスクは間違った抽象化です by Yoshua Wuyts](https://blog.yoshuawuyts.com/tasks-are-the-wrong-abstraction/)を参照
    //
    // 01殿違いの主題はここでjoinはこのブロック内でFutureが解決するまで待つため、ブロック内の生存期間を保証している
    // 'staticが不要
    // ただし動くタスクの数が静的に決まっているパターンでしか使えない
    tokio::try_join!(
        actor_stw.start(token.clone()),
        target.start(token.clone(), actor_tx),
        signal(token),
    )?;
    Ok(())
}
