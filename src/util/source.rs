use std::time::Duration;

use reqwest::Client;
use songbird::{input::AuxMetadata, tracks::Track};

use crate::{
    Sources,
    models::{ApiTrack, ApiTrackInfo, ApiTrackResult},
    source::{http::Http, youtube::Youtube},
};

use super::errors::ResolverError;

pub struct SourceManager {
    pub http: Http,
    pub youtube: Youtube,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            http: Http::new(None),
            youtube: Youtube::new(None),
        }
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        SourceManager::new()
    }
}

pub trait Source {
    fn new(client: Option<Client>) -> Self;

    fn get_client(&self) -> Client;

    async fn valid_url(&self, url: &str) -> bool;

    async fn resolve(&self, url: &str) -> Result<ApiTrackResult, ResolverError>;

    async fn make_playable(&self, track: ApiTrack) -> Result<Track, ResolverError>;
}

impl From<AuxMetadata> for ApiTrackInfo {
    fn from(metadata: AuxMetadata) -> Self {
        let identifier = metadata
            .source_url
            .clone()
            .unwrap_or(String::from("Unknown"));

        let is_seekable = metadata.duration.is_some();
        let author = metadata.artist.unwrap_or(String::from("Unknown"));
        let length = metadata.duration.unwrap_or(Duration::from_millis(u64::MAX));
        let is_stream = length.as_millis() == Duration::from_millis(u64::MAX).as_millis();
        let title = metadata.title.unwrap_or(String::from("Unknown"));

        ApiTrackInfo {
            identifier,
            is_seekable,
            author,
            length: length.as_millis() as u64,
            is_stream,
            position: 0,
            title,
            uri: metadata.source_url,
            artwork_url: metadata.thumbnail,
            isrc: None,
            source_name: String::from("http"),
        }
    }
}

impl ApiTrack {
    pub async fn make_playable(self) -> Result<Track, ResolverError> {
        if self.info.source_name == "http" {
            Sources.http.make_playable(self).await
        } else if self.info.source_name == "youtube" {
            Sources.youtube.make_playable(self).await
        } else {
            return Err(ResolverError::InputNotSupported);
        }
    }
}
