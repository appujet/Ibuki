use std::time::Duration;
use std::{str::FromStr, sync::Arc};

use regex::Regex;
use reqwest::blocking::Client as BlockingClient;
use reqwest::{Body, Client, Url};
use songbird::tracks::Track;
use tokio::sync::Mutex;
use tokio::task::block_in_place;
use tokio::time::Instant;

use crate::util::encoder::encode_base64;
use crate::{
    models::{ApiTrack, ApiTrackInfo, ApiTrackResult},
    util::{errors::ResolverError, source::Source},
};

use super::model::{DeezerApiTrack, DeezerData, DeezerMakePlayableBody, PrivateResponse};
use super::{ARL, PUBLIC_API_BASE, USER_AGENT};
use super::{PRIVATE_API_BASE, SECRET_KEY, model::Tokens};

pub struct Deezer {
    client: Client,
    tokens: Arc<Mutex<Tokens>>,
    regex: Regex,
    search_prefixes: (&'static str, &'static str, &'static str),
}

impl Source for Deezer {
    fn new(client: Option<Client>) -> Self {
        Self {
            client: client.unwrap_or_default(),
            tokens: Arc::new(Mutex::new(Deezer::get_token_blocking())),
            regex: Regex::new("(https?://)?(www\\.)?deezer\\.com/(?<countrycode>[a-zA-Z]{2}/)?(?<type>track|album|playlist|artist)/(?<identifier>[0-9]+)").expect("Failed to init RegEx"),
            search_prefixes: ("dzsearch:", "dzisrc", "dzrec"),
        }
    }

    fn get_name(&self) -> &'static str {
        "deezer"
    }

    fn get_client(&self) -> Client {
        self.client.clone()
    }

    async fn valid_url(&self, url: &str) -> bool {
        Url::from_str(url).ok().is_some() && self.regex.captures(url).is_some()
    }

    async fn try_search(&self, query: &str) -> bool {
        !query.starts_with(self.search_prefixes.0)
            && !query.starts_with(self.search_prefixes.1)
            && !query.starts_with(self.search_prefixes.2)
    }

    async fn search(&self, query: &str) -> Result<ApiTrackResult, ResolverError> {
        let mut data: Option<Vec<DeezerApiTrack>> = None;

        if query.starts_with(self.search_prefixes.0) {
            let term = query.split_at(self.search_prefixes.0.len()).1;

            let query = [("q", term)];

            let request = self
                .client
                .get(format!("{PUBLIC_API_BASE}/search"))
                .query(&query)
                .build()?;

            let response = self.client.execute(request).await?;

            if !response.status().is_success() {
                return Ok(ApiTrackResult::Empty(None));
            }

            let tracks = response.json::<DeezerData<Vec<DeezerApiTrack>>>().await?;

            let _ = data.insert(tracks.data);
        } else if query.starts_with(self.search_prefixes.1) {
            let isrc = query.split_at(self.search_prefixes.1.len()).1;

            let request = self
                .client
                .get(format!("{PUBLIC_API_BASE}/track/isrc:{isrc}"))
                .query(&query)
                .build()?;

            let response = self.client.execute(request).await?;

            if !response.status().is_success() {
                return Ok(ApiTrackResult::Empty(None));
            }

            let _ = data.insert(vec![response.json::<DeezerApiTrack>().await?]);
        }

        let Some(api_tracks) = data else {
            return Ok(ApiTrackResult::Empty(None));
        };

        let tracks = api_tracks
            .iter()
            .filter(|deezer_api_track| deezer_api_track.readable)
            .map(|deezer_api_track| {
                let info = ApiTrackInfo {
                    identifier: deezer_api_track.id.to_string(),
                    is_seekable: true,
                    author: deezer_api_track.artist.name.clone(),
                    length: (deezer_api_track.duration * 1000) as u64,
                    is_stream: false,
                    position: 0,
                    title: deezer_api_track.title.clone(),
                    uri: Some(deezer_api_track.link.clone()),
                    artwork_url: Some(deezer_api_track.album.thumbnail.clone()),
                    isrc: deezer_api_track.isrc.clone(),
                    source_name: self.get_name().to_string(),
                };

                ApiTrack {
                    encoded: encode_base64(&info).unwrap(),
                    info,
                    plugin_info: crate::models::Empty,
                }
            })
            .collect::<Vec<ApiTrack>>();

        Ok(ApiTrackResult::Search(tracks))
    }

    async fn resolve(&self, url: &str) -> Result<ApiTrackResult, ResolverError> {
        todo!()
    }

    async fn make_playable(&self, track: ApiTrack) -> Result<Track, ResolverError> {
        let tokens = {
            let guard = self.tokens.lock().await;
            guard.clone()
        };

        let query = [
            ("method", "song.getData"),
            ("input", "3"),
            ("api_version", "1.0"),
            ("api_token", tokens.check_form.as_str()),
        ];

        let body = DeezerMakePlayableBody {
            sng_id: track.info.identifier.clone(),
        };

        let request = self
            .client
            .post(PRIVATE_API_BASE)
            .header("User-Agent", USER_AGENT)
            .header("Cookie", tokens.create_cookie())
            .body(Body::from(serde_json::to_string(&body)?))
            .query(&query)
            .build()?;

        let response = self.client.execute(request).await?;

        if !response.status().is_success() {
            return Err(ResolverError::FailedStatusCode(
                response.status().to_string(),
            ));
        }

        Err(ResolverError::InputNotSupported)
    }
}

impl Deezer {
    fn get_track_key(id: String) -> [u8; 16] {
        let md5 = hex::encode(md5::compute(id).0);
        let hash = md5.as_bytes();

        let mut key: [u8; 16] = [0; 16];

        for i in 0..16 {
            key[i] = hash[i] ^ hash[i + 16] ^ SECRET_KEY[i];
        }

        key
    }

    /**
     * Used to initialize the source (This panics when it fails)
     */
    fn get_token_blocking() -> Tokens {
        block_in_place(|| {
            let client = BlockingClient::new();

            let query = [
                ("method", "deezer.getUserData"),
                ("input", "3"),
                ("api_version", "1.0"),
                ("api_token", ""),
            ];

            let request = client
                .post(PRIVATE_API_BASE)
                .header("User-Agent", USER_AGENT)
                .header("Content-Length", "0")
                .header("Cookie", format!("arl={ARL}"))
                .query(&query)
                .build()
                .expect("Failed to create a POST request");

            let response = client
                .execute(request)
                .expect("Failed to execute the POST request");

            if !response.status().is_success() {
                panic!("Failed to inititalize Deezer Source: {}", response.status())
            }

            let headers = response
                .headers()
                .get_all("Set-Cookie")
                .iter()
                .map(|header| {
                    String::from(header.to_str().expect("Can\'t convert header to string"))
                })
                .collect::<Vec<String>>();

            let session_id = headers
                .iter()
                .find(|str| str.starts_with("sid="))
                .expect("Session Id Not Found");

            let unique_id = headers
                .iter()
                .find(|str| str.starts_with("dzr_uniq_id="))
                .expect("Unique Id Not Found");

            let data = response
                .json::<PrivateResponse>()
                .expect("Invalid JSON Recieved");

            Tokens {
                session_id: (*session_id).to_string(),
                unique_id: (*unique_id).to_string(),
                check_form: data.results.check_form,
                license_token: data.results.user.options.license_token,
                expire_at: Instant::now()
                    .checked_add(Duration::from_secs(3600))
                    .unwrap(),
            }
        })
    }

    async fn get_token(&self) -> Result<Tokens, ResolverError> {
        let mut guard = self.tokens.lock().await;

        if Instant::now().duration_since(guard.expire_at).as_secs() > 3600 {
            return Ok(guard.clone());
        }

        let query = [
            ("method", "deezer.getUserData"),
            ("input", "3"),
            ("api_version", "1.0"),
            ("api_token", ""),
        ];

        let request = self
            .client
            .post(PRIVATE_API_BASE)
            .header("User-Agent", USER_AGENT)
            .header("Content-Length", "0")
            .header("Cookie", format!("arl={ARL}"))
            .query(&query)
            .build()?;

        let response = self.client.execute(request).await?;

        if !response.status().is_success() {
            return Err(ResolverError::FailedStatusCode(
                response.status().to_string(),
            ));
        }

        let headers = response
            .headers()
            .get_all("Set-Cookie")
            .iter()
            .map(|header| header.to_str())
            .filter(|header| header.is_ok())
            .map(|header| header.unwrap().to_string())
            .collect::<Vec<String>>();

        let session_id = headers.iter().find(|str| str.starts_with("sid=")).ok_or(
            ResolverError::MissingRequiredData("Missing Deezer Session Id"),
        )?;

        let unique_id = headers
            .iter()
            .find(|str| str.starts_with("dzr_uniq_id="))
            .ok_or(ResolverError::MissingRequiredData(
                "Missing Deezer Unique Id",
            ))?;

        let data = response.json::<PrivateResponse>().await?;

        *guard = Tokens {
            session_id: (*session_id).to_string(),
            unique_id: (*unique_id).to_string(),
            check_form: data.results.check_form,
            license_token: data.results.user.options.license_token,
            expire_at: Instant::now()
                .checked_add(Duration::from_secs(3600))
                .ok_or(ResolverError::MissingRequiredData("Invalid Expire At"))?,
        };

        Ok(guard.clone())
    }
}
