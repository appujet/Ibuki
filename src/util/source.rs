use std::time::Duration;

use reqwest::Client;
use songbird::input::{AuxMetadata, Input};

use crate::{models::TrackInfo, source::http::Http};

use super::errors::ResolverError;

pub struct SourceManager {
    pub http: Http,
}

impl SourceManager {
    pub fn new() -> Self {
        Self {
            http: Http::new(None),
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

    fn valid(&self, url: String) -> bool;

    fn get_client(&self) -> Client;

    async fn resolve(&self, url: String) -> Result<TrackInfo, ResolverError>;

    async fn stream(&self, track: &TrackInfo) -> Result<Input, ResolverError>;
}

impl From<AuxMetadata> for TrackInfo {
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

        TrackInfo {
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
