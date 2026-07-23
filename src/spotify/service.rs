use super::client::{PlaybackState, SavedCredentials, SpotifyClient};
use crate::{
    common::{ReadWriteStateHandle, ThreadedReadWriteStateHandle, ThreadedReadonlyStateHandle},
    stats_api::RLEvent,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, RwLock},
    thread,
    time::{Duration, SystemTime},
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpotifySettings {
    pub pause_during_replay: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpotifySavedata {
    credentials: Option<SavedCredentials>,
    settings: SpotifySettings,
}

#[derive(Debug)]
pub enum SpotifyCommand {
    Play,
    Pause,
    Prev,
    Skip,
    Login(String), // client id
    Logout,
}

#[derive(Debug)]
pub struct SpotifyServiceState {
    pub logged_in: bool,
    pub playback_state: Option<PlaybackState>,
    pub last_updated_at: SystemTime,
    pub is_updating: bool,
}

pub const SPOTIFY_REFRESH_INTERVAL: Duration = Duration::from_secs(10);

pub struct SpotifyService {
    client: Arc<RwLock<Option<SpotifyClient>>>,
    settings: ReadWriteStateHandle<SpotifySettings>,
    state: ThreadedReadWriteStateHandle<SpotifyServiceState>,
}

impl SpotifyService {
    pub fn new(savedata: Option<SpotifySavedata>) -> Self {
        let savedata = savedata.unwrap_or_default();

        let client = savedata.credentials.map(SpotifyClient::from_saved);

        SpotifyService {
            settings: ReadWriteStateHandle::new(savedata.settings),
            state: ThreadedReadWriteStateHandle::new(SpotifyServiceState {
                logged_in: client.is_some(),
                playback_state: None,
                last_updated_at: SystemTime::now(),
                is_updating: false,
            }),
            client: Arc::new(RwLock::new(client)),
        }
    }

    pub fn update(&mut self, event: &Arc<Option<RLEvent>>, command: Option<SpotifyCommand>) {
        if let Some(command) = command {
            self.handle_command(command);
        }

        {
            let settings = self.settings.read();
            if settings.pause_during_replay
                && let Some(event) = event.as_ref()
            {
                drop(settings);
                match event {
                    RLEvent::ReplayStart | RLEvent::MatchOver(_) => {
                        self.handle_command(SpotifyCommand::Pause);
                    }
                    RLEvent::ReplayDone | RLEvent::MatchLeft => {
                        self.handle_command(SpotifyCommand::Play);
                    }
                    _ => {}
                }
            }
        }

        let state = self.state.read();
        if SystemTime::now()
            .duration_since(state.last_updated_at)
            .unwrap_or_default()
            > SPOTIFY_REFRESH_INTERVAL
        {
            self.use_client_then_update_playback(|_| {});
        }
    }

    pub fn handle_command(&mut self, command: SpotifyCommand) {
        match command {
            SpotifyCommand::Login(client_id) => {
                if self.client.read().unwrap().is_some() {
                    return;
                }

                let client_ref = Arc::clone(&self.client);
                let state_ref = ThreadedReadWriteStateHandle::clone(&self.state);
                thread::spawn(move || {
                    let new_client = SpotifyClient::from_scratch(client_id);
                    let mut old_client = client_ref.write().unwrap();
                    *old_client = Some(new_client);
                    state_ref.write().logged_in = true;
                });
            }
            SpotifyCommand::Logout => {
                let mut client = self.client.write().unwrap();
                *client = None;
                let mut state = self.state.write();
                state.logged_in = false;
            }
            SpotifyCommand::Play => {
                self.use_client_then_update_playback(SpotifyClient::unpause_playback);
            }
            SpotifyCommand::Pause => {
                self.use_client_then_update_playback(SpotifyClient::pause_playback);
            }
            SpotifyCommand::Skip => {
                self.use_client_then_update_playback(SpotifyClient::skip_song);
            }
            SpotifyCommand::Prev => {
                self.use_client_then_update_playback(SpotifyClient::prev_song);
            }
        }
    }

    pub fn save(&self) -> SpotifySavedata {
        let client = &*self.client.read().unwrap();
        SpotifySavedata {
            credentials: client.as_ref().map(SpotifyClient::save),
            settings: self.settings.read().clone(),
        }
    }

    fn use_client_then_update_playback<F>(&self, fun: F)
    where
        F: Fn(&SpotifyClient) + Send + 'static,
    {
        let client_ref = Arc::clone(&self.client);
        let state_ref = ThreadedReadWriteStateHandle::clone(&self.state);

        thread::spawn(move || {
            {
                let mut state = state_ref.write();
                state.is_updating = true;
                state.last_updated_at = SystemTime::now();
            }

            let client = &*client_ref.read().unwrap();
            if let Some(client) = client {
                fun(client);

                thread::sleep(Duration::from_secs(1));
                let new_playback_state = client.get_playback_state();
                let mut state = state_ref.write();
                state.playback_state = new_playback_state;
                state.is_updating = false;
            }
        });
    }

    pub fn state_handle(&self) -> ThreadedReadonlyStateHandle<SpotifyServiceState> {
        ThreadedReadonlyStateHandle::over(&self.state)
    }

    pub fn settings_handle(&self) -> ReadWriteStateHandle<SpotifySettings> {
        ReadWriteStateHandle::clone(&self.settings)
    }
}
