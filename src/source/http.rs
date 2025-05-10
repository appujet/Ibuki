use std::str::FromStr;

use reqwest::{Client, Url};
use songbird::input::{AuxMetadata, Compose, HttpRequest, Input, LiveInput};

use crate::{
    models::{DataType, Track, TrackInfo},
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

    fn get_client(&self) -> Client {
        self.client.clone()
    }

    async fn valid_url(&self, url: &str) -> bool {
        Url::from_str(url).ok().is_some()
    }

    async fn resolve(&self, url: &str) -> Result<DataType, ResolverError> {
        let mut request = HttpRequest::new(self.get_client(), url.to_string());

        let mut metadata = request
            .aux_metadata()
            .await
            .unwrap_or(AuxMetadata::default());

        if metadata.source_url.is_none() {
            let _ = metadata.source_url.insert(url.to_owned());
        }

        let info: TrackInfo = metadata.into();

        let track = Track {
            encoded: encode_base64(&info)?,
            info,
            plugin_info: serde_json::Value::Null,
        };

        Ok(DataType::Track(track))
    }

    async fn stream(&self, track: &TrackInfo) -> Result<Input, ResolverError> {
        let mut request = HttpRequest::new(
            self.get_client(),
            track
                .uri
                .clone()
                .ok_or(ResolverError::MissingRequiredData("uri"))?,
        );

        let stream = request.create_async().await?;
        let input = Input::Live(LiveInput::Raw(stream), None);

        Ok(input)
    }
}
