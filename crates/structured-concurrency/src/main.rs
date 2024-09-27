use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

struct Actor {
    position: f32,
    velocity: f32,
    sender_queue: Vec<mpsc::Sender<f32>>,
}

impl Actor {
    fn new(position: f32, velocity: f32) -> Self {
        Self {
            position,
            velocity,
            sender_queue: Vec::new(),
        }
    }

    fn update(&mut self, dt: f32) {
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

    fn get_position(&self) -> f32 {
        self.position
    }

    fn set_velocity(&mut self, velocity: f32) {
        self.velocity = velocity;
    }
}

impl StActor for Actor {
    type Msg = ActorIn;
    async fn recv(&mut self, rx: &mut mpsc::Receiver<Self::Msg>) {
        while let Some(in_msg) = rx.try_recv().ok() {
            match in_msg {
                ActorIn::SetVel(vel) => self.set_velocity(vel),
                ActorIn::PosReader(tx) => {
                    self.sender_queue.push(tx);
                }
            }
        }
    }

    async fn start(&mut self, token: CancellationToken, rx: &mut mpsc::Receiver<Self::Msg>) {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            self.update(0.1);
            self.recv(rx).await;
            tokio::select! {
                _ = token.cancelled() => {
                    break;
                }
                _ = interval.tick() => {}
            }
        }
    }
}

enum ActorIn {
    SetVel(f32),
    PosReader(mpsc::Sender<f32>),
}

trait StActor {
    type Msg;
    async fn recv(&mut self, rx: &mut mpsc::Receiver<Self::Msg>);
    async fn start(&mut self, token: CancellationToken, rx: &mut mpsc::Receiver<Self::Msg>);
}

struct Stw<T, In> {
    state: T,
    in_tx: mpsc::Sender<In>,
    in_rx: mpsc::Receiver<In>,
}

impl<T, In> Stw<T, In> {
    fn new(state: T) -> Self {
        let (in_tx, in_rx) = mpsc::channel(10);
        Self {
            state,
            in_tx,
            in_rx,
        }
    }

    fn tx(&self) -> mpsc::Sender<In> {
        self.in_tx.clone()
    }
}

impl<T, In> Stw<T, In>
where
    T: StActor<Msg = In>,
{
    async fn recv(&mut self) {
        self.state.recv(&mut self.in_rx).await;
    }

    async fn start(&mut self, token: CancellationToken) {
        println!("start_task");
        self.state.start(token, &mut self.in_rx).await;
        println!("shutdown");
    }
}

impl<T, In> AsRef<T> for Stw<T, In> {
    fn as_ref(&self) -> &T {
        &self.state
    }
}

impl<T, In> AsMut<T> for Stw<T, In> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.state
    }
}

struct Target {
    position: f32,
    vel_max: f32,
    gain: f32,
    epsilon: f32,
}

impl Target {
    fn new(position: f32, vel_max: f32) -> Self {
        Self {
            position,
            vel_max,
            gain: 1.0,
            epsilon: 0.01,
        }
    }

    fn calc_vel(&self, current_pos: f32) -> f32 {
        let diff = (self.position - current_pos) * self.gain;
        if diff.abs() < self.epsilon {
            0.0
        } else {
            diff.clamp(-self.vel_max, self.vel_max)
        }
    }

    async fn start(&mut self, token: CancellationToken, tx_act: mpsc::Sender<ActorIn>) {
        let mut interval = tokio::time::interval(Duration::from_millis(200));
        let (tx, mut rx) = mpsc::channel(10);
        tx_act.send(ActorIn::PosReader(tx)).await.unwrap();
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
                    tx_act.try_send(ActorIn::SetVel(vel)).unwrap();
                }
            }
        }
    }
}

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_with_join()
}

// joinを使う場合
// こちらはblock無いで生存していることを必須としているので実行可能
fn run_with_join() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(run_with_join_inner())?;
    Ok(())
}

async fn signal(token: CancellationToken) {
    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    token.cancel();
}

async fn run_with_join_inner() -> Result<(), Box<dyn std::error::Error>> {
    let token = CancellationToken::new();
    let actor = Actor::new(0.0, 1.0);
    let mut actor_stw = Stw::new(actor);
    let actor_tx: mpsc::Sender<ActorIn> = actor_stw.tx();
    let mut target = Target::new(10.0, 1.0);

    // ここはfutures::join!でもよい
    tokio::join!(
        actor_stw.start(token.clone()),
        target.start(token.clone(), actor_tx),
        signal(token),
    );
    Ok(())
}

// localset spawn_localを使う場合
// spawn_localは'static境界を持つため async move が必要
fn run_with_spawn() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        println!("hello from the main future");
        let local = tokio::task::LocalSet::new();
        let actor = Actor::new(0.0, 1.0);
        let mut actor_stw = Stw::new(actor);
        let actor_tx: mpsc::Sender<ActorIn> = actor_stw.tx();

        let token = CancellationToken::new();
        let token_h0 = token.clone();
        let _h0 = local.spawn_local(async move {
            tokio::signal::ctrl_c().await.unwrap();
            println!("Ctrl-C received, shutting down");
            token_h0.cancel();
        });

        let token_h1 = token.clone();
        let _h1 = local.spawn_local(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
            loop {
                actor_stw.as_mut().update(0.1);
                actor_stw.recv().await;
                tokio::select! {
                    _ = token_h1.cancelled() => {
                        break;
                    }
                    _ = interval.tick() => {}
                }
            }
            println!("shutdown h1");
        });

        let token_h2 = token.clone();
        let _h2 = local.spawn_local(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
            let (tx, mut rx) = mpsc::channel(10);
            actor_tx.send(ActorIn::PosReader(tx)).await.unwrap();
            let target = Target::new(10.0, 1.0);
            let mut last_pos = 0.0;
            loop {
                while let Some(pos) = rx.try_recv().ok() {
                    last_pos = pos;
                }
                let vel = target.calc_vel(last_pos);
                println!("Actor position from reader: {last_pos} -> {vel}");
                actor_tx.try_send(ActorIn::SetVel(vel)).unwrap();

                tokio::select! {
                    _ = token_h2.cancelled() => {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        break;
                    }
                    _ = interval.tick() => {}
                }
            }
            println!("shutdown h2");
        });

        // spawnでなくとも、ここにjoinできるならasync moveもいらないのでは...?
        local.await;

        println!("graceful shutdown");
    });
    Ok(())
}
