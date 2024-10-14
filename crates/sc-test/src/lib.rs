use std::time::Duration;

use anyhow::Context;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub mod error;

// 独自にループ処理を含む実行フローを持つ処理の例
// このアクターの場合は自身の速度を元に経時変化で位置を更新する
// 時間定期な経過による待ち時間が主でCPU処理としては極小
pub struct Actor {
    position: f32,
    velocity: f32,
    sender_queue: Vec<mpsc::Sender<f32>>,
}

impl Actor {
    pub fn new(position: f32, velocity: f32) -> Self {
        Self {
            position,
            velocity,
            sender_queue: Vec::new(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.position += self.velocity * dt;
        for tx in self.sender_queue.iter() {
            if tx.is_closed() {
                continue;
            }
            match tx.try_send(self.position) {
                Ok(_) => {}
                Err(e) => {
                    println!("Failed to send position {}", e);
                }
            }
        }
        self.sender_queue.retain(|tx| !tx.is_closed());
    }

    pub fn get_position(&self) -> f32 {
        self.position
    }

    pub fn set_velocity(&mut self, velocity: f32) {
        self.velocity = velocity;
    }
}

impl StActor for Actor {
    type Msg = ActorIn;
    type Error = crate::error::Error;
    async fn recv(&mut self, rx: &mut mpsc::Receiver<Self::Msg>) -> Result<(), Self::Error> {
        while let Ok(in_msg) = rx.try_recv() {
            match in_msg {
                ActorIn::SetVel(vel) => self.set_velocity(vel),
                ActorIn::PosReader(tx) => {
                    self.sender_queue.push(tx);
                }
            }
        }
        Ok(())
    }

    async fn start(
        &mut self,
        token: CancellationToken,
        rx: &mut mpsc::Receiver<Self::Msg>,
    ) -> Result<(), Self::Error> {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            self.update(0.1);
            self.recv(rx).await?;
            tokio::select! {
                _ = token.cancelled() => {
                    break;
                }
                _ = interval.tick() => {}
            }
        }
        println!("Actor shutdown");
        Ok(())
    }
}

// 今回のアクターはイベント駆動で記述しているので、メッセージの種類を列挙しておく
pub enum ActorIn {
    SetVel(f32),
    PosReader(mpsc::Sender<f32>),
}

// アクターのトレイト。
// 処理の起動方法とメッセージの受信処理を定義
// 状態を持つアクターを保持してそれを更新する
pub trait StActor {
    type Msg;
    type Error;

    fn recv(
        &mut self,
        rx: &mut mpsc::Receiver<Self::Msg>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>>;
    // キャンセラブルにするためにトークンを渡している。
    // 実装者はこれを保証する必要があるが特性的な制限をしていない
    fn start(
        &mut self,
        token: CancellationToken,
        rx: &mut mpsc::Receiver<Self::Msg>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>>;
}

// アクターに対してメッセージを送受信する口を提供するラッパー
// 動的に非同期処理が増える場合はこのようなラッパーが必要になりそうなので定義
pub struct StWrapper<T, In> {
    state: T,
    in_tx: mpsc::Sender<In>,
    in_rx: mpsc::Receiver<In>,
}

impl<T, In> StWrapper<T, In> {
    pub fn new(state: T) -> Self {
        let (in_tx, in_rx) = mpsc::channel(10);
        Self {
            state,
            in_tx,
            in_rx,
        }
    }

    // senderを渡すことでmpscな関係を作れる
    pub fn tx(&self) -> mpsc::Sender<In> {
        self.in_tx.clone()
    }
}

impl<T, In> StWrapper<T, In>
where
    T: StActor<Msg = In, Error = crate::error::Error>,
{
    pub async fn recv(&mut self) -> Result<(), T::Error> {
        self.state.recv(&mut self.in_rx).await
    }

    pub async fn start(&mut self, token: CancellationToken) -> Result<(), T::Error> {
        println!("start_task");
        self.state.start(token, &mut self.in_rx).await?;
        println!("shutdown");
        Ok(())
    }
}

impl<T, In> AsRef<T> for StWrapper<T, In> {
    fn as_ref(&self) -> &T {
        &self.state
    }
}

impl<T, In> AsMut<T> for StWrapper<T, In> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.state
    }
}

// Actor向けの制御ロジック
pub struct Target {
    position: f32,
    vel_max: f32,
    gain: f32,
    epsilon: f32,
}

impl Target {
    pub fn new(position: f32, vel_max: f32) -> Self {
        Self {
            position,
            vel_max,
            gain: 1.0,
            epsilon: 0.01,
        }
    }

    pub fn calc_vel(&self, current_pos: f32) -> f32 {
        let diff = (self.position - current_pos) * self.gain;
        if diff.abs() < self.epsilon {
            0.0
        } else {
            diff.clamp(-self.vel_max, self.vel_max)
        }
    }

    // こちらも同様に非同期ループを実行する構造
    pub async fn start(
        &mut self,
        token: CancellationToken,
        tx_act: mpsc::Sender<ActorIn>,
    ) -> crate::error::Result<()> {
        let mut interval = tokio::time::interval(Duration::from_millis(200));
        let (tx, mut rx) = mpsc::channel(10);
        tx_act
            .send(ActorIn::PosReader(tx))
            .await
            .context("start up message")?;
        let mut current_pos = 0.0;
        loop {
            // futures::select! はFusedFutureを要求するので、ここで代替はできない
            // 分岐に関してはRuntimeに寄せるほうが望ましいのかもしれない
            tokio::select! {
                _ = token.cancelled() => {
                    break;
                }
                x = rx.recv() => {
                    match x {
                        Some(pos) => current_pos = pos,
                        None => {
                            println!("pos reader closed");
                            token.cancel();
                            break;
                        }
                    }
                }
                _ = interval.tick() => {
                    let vel = self.calc_vel(current_pos);
                    println!("Actor position from reader: {current_pos} -> {vel}");
                    tx_act.send(ActorIn::SetVel(vel)).await.context("send message")?;
                }
            }
        }
        // 終了時の状態を定義
        tx_act
            .send(ActorIn::SetVel(0.0))
            .await
            .context("closing message")?;
        println!("Target shutdown");
        Ok(())
    }
}

/// Ctrl-Cを受信してキャンセルトークンをキャンセルする
pub async fn signal(token: CancellationToken) -> crate::error::Result<()> {
    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    token.cancel();
    Ok(())
}
