use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, Timestamps},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceData {
    pub details: String,
    pub state: Option<String>,
}

struct RichPresence {
    previous_send: Option<PresenceData>,
    client: DiscordIpcClient,
    is_connected: bool,
    start_time: i64,
    last_connection_attempt_at: SystemTime,
}

const APP_ID: &str = "356877880938070016";
const MIN_CONNECTION_INTERVAL: Duration = Duration::from_secs(5);

impl RichPresence {
    pub fn new() -> RichPresence {
        RichPresence {
            client: DiscordIpcClient::new(APP_ID),
            previous_send: None,
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .try_into()
                .unwrap(),
            is_connected: false,
            last_connection_attempt_at: UNIX_EPOCH,
        }
    }

    pub fn ensure_connected(&mut self) {
        if SystemTime::now()
            .duration_since(self.last_connection_attempt_at)
            .unwrap()
            < MIN_CONNECTION_INTERVAL
        {
            return;
        }

        if self.is_connected {
            return;
        }

        self.last_connection_attempt_at = SystemTime::now();
        self.is_connected = true;

        if self.client.connect().is_err() {
            self.is_connected = false;
        }
    }

    pub fn ensure_disconnected(&mut self) {
        if !self.is_connected {
            return;
        }

        let _ = self.client.clear_activity();
        let _ = self.client.close();
        self.previous_send = None;
        self.is_connected = false;
    }

    pub fn send(&mut self, presence: PresenceData) {
        if !self.is_connected {
            return;
        }

        if Some(&presence) == self.previous_send.as_ref() {
            return;
        }

        let mut activity = Activity::new()
            .timestamps(Timestamps::new().start(self.start_time))
            .details(&presence.details);

        if let Some(state) = presence.state.as_ref() {
            activity = activity.state(state);
        }

        self.client.set_activity(activity).unwrap();
        self.previous_send = Some(presence);
    }
}

enum RPCCommand {
    Connect,
    Disconnect,
    Set(PresenceData),
}

fn create_controller_thread(
    rx: mpsc::Receiver<RPCCommand>,
    connected: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let mut rpc = RichPresence::new();

        loop {
            if let Ok(command) = rx.recv() {
                match command {
                    RPCCommand::Connect => {
                        busy.store(!rpc.is_connected, Ordering::Relaxed);
                        rpc.ensure_connected();
                        connected.store(rpc.is_connected, Ordering::Relaxed);
                        busy.store(!rpc.is_connected, Ordering::Relaxed);
                    }
                    RPCCommand::Disconnect => {
                        busy.store(true, Ordering::Relaxed);
                        rpc.ensure_disconnected();
                        connected.store(false, Ordering::Relaxed);
                        busy.store(false, Ordering::Relaxed);
                    }
                    RPCCommand::Set(act) => {
                        rpc.send(act);
                    }
                }
            }
        }
    });
}

pub struct RichPresenceController {
    tx: mpsc::Sender<RPCCommand>,
    connected: Arc<AtomicBool>,
    busy: Arc<AtomicBool>,
}

impl RichPresenceController {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let connected = Arc::new(AtomicBool::from(false));
        let busy = Arc::new(AtomicBool::from(false));

        create_controller_thread(rx, Arc::clone(&connected), Arc::clone(&busy));

        RichPresenceController {
            tx,
            connected,
            busy,
        }
    }

    pub fn ensure_connected(&self) {
        self.send(RPCCommand::Connect);
    }

    pub fn ensure_disconnected(&self) {
        self.send(RPCCommand::Disconnect);
    }

    pub fn set_presence(&self, data: PresenceData) {
        self.send(RPCCommand::Set(data));
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub fn is_busy(&self) -> bool {
        self.busy.load(Ordering::Relaxed)
    }

    fn send(&self, command: RPCCommand) {
        if let Err(error) = self.tx.send(command) {
            println!("[rpc main thread] channel send error: {error:?}");
        }
    }
}
