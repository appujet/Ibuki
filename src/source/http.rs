use std::{str::FromStr, sync::Arc, time::Duration};

use reqwest::{Client, Url};
use songbird::{
    input::{AuxMetadata, Compose, HttpRequest, Input, LiveInput},
    tracks::Track,
};

use crate::{
    models::{ApiTrack, ApiTrackInfo, ApiTrackResult},
    util::{encoder::encode_base64, errors::ResolverError, source::Source},
};

pub struct Http {
    client: Client,
}

impl Source for Http {
    fn new(client: Option<Client>) -> Self {
        Self {
            client: client.unwrap_or_default(),
        }
    }

    fn get_name(&self) -> &'static str {
        "http"
    }

    fn get_client(&self) -> Client {
        self.client.clone()
    }

    async fn valid_url(&self, url: &str) -> bool {
        Url::from_str(url).ok().is_some()
    }

    async fn try_search(&self, _: &str) -> bool {
        false
    }

    async fn search(&self, _: &str) -> Result<ApiTrackResult, ResolverError> {
        Err(ResolverError::InputNotSupported)
    }

    async fn resolve(&self, url: &str) -> Result<ApiTrackResult, ResolverError> {
        let client = self.get_client();
        let response = client.get(url).send().await?;

        let content = response
            .headers()
            .get("Content-Type")
            .ok_or(ResolverError::InputNotSupported)?;

        if !content.to_str()?.contains("audio") {
            return Err(ResolverError::InputNotSupported);
        }

        let mut request = HttpRequest::new(self.get_client(), url.to_string());

        let mut metadata = request
            .aux_metadata()
            .await
            .unwrap_or(AuxMetadata::default());

        if metadata.source_url.is_none() {
            let _ = metadata.source_url.insert(url.to_owned());
        }

        let info = self.make_track(metadata);

        let track = ApiTrack {
            encoded: encode_base64(&info)?,
            info,
            plugin_info: serde_json::Value::Null,
        };

        Ok(ApiTrackResult::Track(track))
    }

    async fn make_playable(&self, track: ApiTrack) -> Result<Track, ResolverError> {
        let mut request = HttpRequest::new(
            self.get_client(),
            track
                .info
                .uri
                .clone()
                .ok_or(ResolverError::MissingRequiredData("uri"))?,
        );

        let stream = request.create_async().await?;
        let input = Input::Live(LiveInput::Raw(stream), None);

        Ok(Track::new_with_data(input, Arc::new(track)))
    }
}

impl Http {
    fn make_track(&self, metadata: AuxMetadata) -> ApiTrackInfo {
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
            source_name: self.get_name().into(),
        }
    }
}
