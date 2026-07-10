use std::{
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
    thread,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};

use crate::rocket_league::RLEvent;

use super::client::{Client, PlaybackState, SavedCredentials};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpotifySettings {
    pub pause_during_replay: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpotifySavedata {
    credentials: Option<SavedCredentials>,
    settings: Rc<SpotifySettings>,
}

#[derive(Debug)]
pub enum SpotifyCommand {
    Play,
    Pause,
    Skip,
    Login,
    Logout,
    UpdateSettings(SpotifySettings),
}

#[derive(Debug)]
pub struct SpotifyServiceState {
    pub logged_in: bool,
    pub playback_state: Arc<Mutex<Option<PlaybackState>>>,
    pub last_updated_at: SystemTime,
    pub settings: Rc<SpotifySettings>,
}

pub const SPOTIFY_REFRESH_INTERVAL: Duration = Duration::from_secs(10);

pub struct SpotifyService {
    client: Arc<RwLock<Option<Client>>>,
    settings: Rc<SpotifySettings>,

    currently_playing: Arc<Mutex<Option<PlaybackState>>>,
    last_updated_at: SystemTime,
}

impl SpotifyService {
    pub fn new(savedata: Option<SpotifySavedata>) -> Self {
        let savedata = savedata.unwrap_or_default();
        let client = Arc::new(RwLock::new(savedata.credentials.map(Client::from_saved)));

        SpotifyService {
            client,
            settings: savedata.settings,
            currently_playing: Arc::new(Mutex::new(None)),
            last_updated_at: SystemTime::now(),
        }
    }

    pub fn update(
        &mut self,
        event: &Arc<Option<RLEvent>>,
        command: Option<SpotifyCommand>,
    ) -> SpotifyServiceState {
        if let Some(command) = command {
            self.handle_command(command);
        }

        if self.settings.pause_during_replay
            && let Some(event) = event.as_ref()
        {
            match event {
                RLEvent::ReplayStart => self.handle_command(SpotifyCommand::Pause),
                RLEvent::ReplayDone => self.handle_command(SpotifyCommand::Play),
                _ => {}
            }
        }

        if SystemTime::now()
            .duration_since(self.last_updated_at)
            .unwrap_or_default()
            > SPOTIFY_REFRESH_INTERVAL
        {
            self.last_updated_at = SystemTime::now();

            let playback_state = Arc::clone(&self.currently_playing);
            self.use_client_in_new_thread(move |client| {
                let new_state = client.get_playback_state();
                let mut playback_state = playback_state.lock().unwrap();
                *playback_state = new_state;
            });
        }

        SpotifyServiceState {
            logged_in: self.client.read().unwrap().is_some(),
            playback_state: Arc::clone(&self.currently_playing),
            last_updated_at: self.last_updated_at,
            settings: Rc::clone(&self.settings),
        }
    }

    pub fn handle_command(&mut self, command: SpotifyCommand) {
        match command {
            SpotifyCommand::Login => {
                if self.client.read().unwrap().is_some() {
                    return;
                }

                let client_ref = Arc::clone(&self.client);
                thread::spawn(move || {
                    let new_client = Client::from_scratch();
                    let mut old_client = client_ref.write().unwrap();
                    *old_client = Some(new_client);
                });
            }
            SpotifyCommand::Logout => {
                let mut client = self.client.write().unwrap();
                *client = None;
            }
            SpotifyCommand::Play => {
                self.use_client_in_new_thread(Client::unpause_playback);
            }
            SpotifyCommand::Pause => {
                self.use_client_in_new_thread(Client::pause_playback);
            }
            SpotifyCommand::Skip => {
                self.use_client_in_new_thread(Client::skip_song);
            }
            SpotifyCommand::UpdateSettings(new_settings) => {
                self.settings = Rc::new(new_settings);
            }
        }
    }

    pub fn save(&self) -> SpotifySavedata {
        let client = &*self.client.read().unwrap();
        SpotifySavedata {
            credentials: client.as_ref().map(Client::save),
            settings: self.settings.clone(),
        }
    }

    fn use_client_in_new_thread<F>(&self, fun: F)
    where
        F: Fn(&Client) + Send + 'static,
    {
        let client_ref = Arc::clone(&self.client);
        thread::spawn(move || {
            let client = &*client_ref.read().unwrap();
            if let Some(client) = client {
                fun(client);
            }
        });
    }
}
