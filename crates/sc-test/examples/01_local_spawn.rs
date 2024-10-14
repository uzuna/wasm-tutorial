//! LocalSet spawn_localを使う実装例
//!
//! LocalSetの生存期間に囚われて入るがspawn_localは非構造化並行性を提供している
//! Futureは全て間勝利なくても終了が可能になっている

use sc_test::{signal, Actor, ActorIn, StWrapper, Target};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub fn main() -> anyhow::Result<()> {
    run_with_spawn()
}

fn run_with_spawn() -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        println!("hello from the main future");
        let local = tokio::task::LocalSet::new();
        let actor = Actor::new(0.0, 1.0);
        let mut actor_stw = StWrapper::new(actor);
        let actor_tx: mpsc::Sender<ActorIn> = actor_stw.tx();

        // シグナル受信と停止の生成
        let token = CancellationToken::new();
        let _h0 = local.spawn_local(signal(token.clone()));

        // 制御対象の独自ループを動かすタスクの生成
        let token_h1 = token.clone();
        // 01の課題としてasync moveが必要になること
        // spawnはこのブロックを抜けても実行可能であるため、生存期間を保証するためにasync move = 'staticを必要としている
        let _h1 = local.spawn_local(async move { actor_stw.start(token_h1).await });

        // 制御器の独自ループを動かすタスクの生成
        let mut target = Target::new(10.0, 1.0);
        let _h2 = local.spawn_local(async move { target.start(token, actor_tx).await });

        // すべてのタスクが終了するまで待つ
        local.await;

        println!("graceful shutdown");
    });
    Ok(())
}
