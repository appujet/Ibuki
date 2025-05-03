use std::str::FromStr;

use async_trait::async_trait;
use reqwest::{Client, Url};
use songbird::input::{AudioStream, AuxMetadata, Compose, HttpRequest};
use symphonia::core::io::MediaSource;

use crate::{
    models::TrackInfo,
    util::{errors::ResolverError, source::Source},
};

pub struct Http {
    client: Client,
}

#[async_trait]
impl Source for Http {
    fn valid(&self, url: String) -> bool {
        Url::from_str(&url).ok().is_some()
    }

    fn get_client(&self) -> Client {
        self.client.clone()
    }

    async fn resolve(&self, url: String) -> Result<TrackInfo, ResolverError> {
        let mut request = HttpRequest::new(self.get_client(), url.to_string());

        let mut metadata = request
            .aux_metadata()
            .await
            .unwrap_or(AuxMetadata::default());

        if metadata.source_url.is_none() {
            let _ = metadata.source_url.insert(url);
        }

        return Ok(metadata.into());
    }

    async fn stream(
        &self,
        track: &TrackInfo,
    ) -> Result<AudioStream<Box<dyn MediaSource>>, ResolverError> {
        let mut request = HttpRequest::new(
            self.get_client(),
            track
                .uri
                .clone()
                .ok_or(ResolverError::MissingRequiredData("uri"))?,
        );

        Ok(request.create_async().await?)
    }
}

impl Http {
    pub fn new(client: Option<Client>) -> Self {
        Self {
            client: client.unwrap_or_default(),
        }
    }
}
