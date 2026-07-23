use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::distr::SampleString;
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::io::{Read as _, Write};
use std::net::TcpListener;

const REDIRECT_URL: &str = "http://127.0.0.1:7742/";

fn sha256(input: &str) -> Vec<u8> {
    Sha256::digest(input).into_iter().collect::<Vec<u8>>()
}

fn generate_code_verifier() -> String {
    rand::distr::Alphanumeric.sample_string(&mut rand::rng(), 64)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedCredentials {
    refresh_token: String,
    client_id: String,
}

#[derive(Deserialize)]
pub struct RefreshFlowResponse {
    access_token: String,
}

#[derive(Deserialize)]
pub struct AuthFlowResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct SpotifyClient {
    client_id: String,
    access_token: String,
    refresh_token: String,
}

const AUTH_CODE_REDIRECT_PAGE_CONTENT: &str = r#"<!DOCTYPE html>
<h1>authorization complete!!!</h1>
<p>this tab will close in <strong>3</strong> <span>seconds</span></p>
<script>
let val = 3;
setInterval(() => {
    val--;
    document.querySelector("strong").innerText = val;
    document.querySelector("span").innerText = `second${val == 1 ? '' : 's'}`;
    if (val == 0) window.close();
}, 1000);
</script>
"#;

impl SpotifyClient {
    pub fn save(&self) -> SavedCredentials {
        SavedCredentials {
            refresh_token: self.refresh_token.clone(),
            client_id: self.client_id.clone(),
        }
    }

    pub fn from_scratch(client_id: String) -> SpotifyClient {
        let verifier = generate_code_verifier();
        let hashed = sha256(&verifier);
        let code_challenge = URL_SAFE_NO_PAD.encode(hashed);

        let url = format!(
            "https://accounts.spotify.com/authorize\
            ?response_type=code\
            &client_id={client_id}\
            &scope={}\
            &code_challenge_method=S256\
            &code_challenge={code_challenge}\
            &redirect_uri={}",
            urlencoding::encode("user-read-playback-state user-modify-playback-state"),
            urlencoding::encode(REDIRECT_URL)
        );

        webbrowser::open(&url).unwrap();

        // temporary small http server
        let listener = TcpListener::bind("127.0.0.1:7742").unwrap();
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0; 1024];
        stream.read_exact(&mut buffer).unwrap();
        let response = String::from_utf8_lossy(&buffer);

        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\n\
                    Content-Type: text/html; charset=utf-8\r\n\
                    Content-Length: {}\r\n\
                    Connection: close\r\n\r\n\
                    {}",
                    AUTH_CODE_REDIRECT_PAGE_CONTENT.len(),
                    AUTH_CODE_REDIRECT_PAGE_CONTENT
                )
                .as_bytes(),
            )
            .unwrap();
        stream.flush().unwrap();

        let authorization_code = response
            .split_once("?code=")
            .unwrap()
            .1
            .split_once("&ubi=")
            .unwrap()
            .0;
        let authorization_code = urlencoding::decode(authorization_code)
            .unwrap()
            .into_owned();

        let form = [
            ("grant_type", "authorization_code"),
            ("client_id", &client_id),
            ("code", &authorization_code),
            ("redirect_uri", REDIRECT_URL),
            ("code_verifier", &verifier),
        ];

        let resp = ureq::post("https://accounts.spotify.com/api/token")
            .send_form(form)
            .unwrap()
            .body_mut()
            .read_json::<AuthFlowResponse>()
            .unwrap();

        SpotifyClient {
            client_id,
            access_token: resp.access_token,
            refresh_token: resp.refresh_token,
        }
    }

    pub fn from_saved(credentials: SavedCredentials) -> SpotifyClient {
        let form = [
            ("grant_type", "refresh_token"),
            ("client_id", &credentials.client_id),
            ("refresh_token", &credentials.refresh_token),
        ];

        let response: RefreshFlowResponse = ureq::post("https://accounts.spotify.com/api/token")
            .send_form(form)
            .unwrap()
            .body_mut()
            .read_json()
            .unwrap();

        SpotifyClient {
            access_token: response.access_token,
            refresh_token: credentials.refresh_token,
            client_id: credentials.client_id,
        }
    }

    pub fn get_playback_state(&self) -> Option<PlaybackState> {
        let r = ureq::get("https://api.spotify.com/v1/me/player")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .call()
            .unwrap();

        if r.status() == ureq::http::StatusCode::NO_CONTENT {
            return None;
        }

        r.into_body().read_json().unwrap()
    }

    pub fn prev_song(&self) {
        if let Err(error) = ureq::post("https://api.spotify.com/v1/me/player/previous")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send_empty()
        {
            println!("[previous] spotify api error: {error:?}");
        }
    }

    pub fn skip_song(&self) {
        if let Err(error) = ureq::post("https://api.spotify.com/v1/me/player/next")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send_empty()
        {
            println!("[next] spotify api error: {error:?}");
        }
    }

    pub fn pause_playback(&self) {
        if let Err(error) = ureq::put("https://api.spotify.com/v1/me/player/pause")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send_empty()
        {
            println!("[pause] spotify api error: {error:?}");
        }
    }

    pub fn unpause_playback(&self) {
        if let Err(error) = ureq::put("https://api.spotify.com/v1/me/player/play")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send_empty()
        {
            println!("[play] spotify api error: {error:?}");
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Artist {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Track {
    pub name: String,
    pub artists: Vec<Artist>,
}

#[derive(Debug, Deserialize)]
pub struct PlaybackState {
    #[serde(rename = "item")]
    pub track: Track,
}
