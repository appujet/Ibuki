use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use songbird::input::{AudioStream, AuxMetadata};
use symphonia::core::io::MediaSource;

use crate::models::TrackInfo;

use super::errors::ResolverError;

#[async_trait]
pub trait Source {
    fn valid(&self, url: String) -> bool;

    fn get_client(&self) -> Client;

    async fn resolve(&self, url: String) -> Result<TrackInfo, ResolverError>;

    async fn stream(
        &self,
        track: &TrackInfo,
    ) -> Result<AudioStream<Box<dyn MediaSource>>, ResolverError>;
}

impl From<AuxMetadata> for TrackInfo {
    fn from(metadata: AuxMetadata) -> Self {
        let identifier = metadata
            .source_url
            .clone()
            .unwrap_or(String::from("Unknown"));

        let is_seekable = metadata.duration.is_some();
        let author = metadata.artist.unwrap_or(String::from("Unknown"));
        let length = metadata.duration.unwrap_or(Duration::from_secs(0));
        let is_stream = metadata.duration.is_none();
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
