use std::str::FromStr;

use reqwest::{Client, Url};
use songbird::input::{AuxMetadata, Compose, HttpRequest, Input};

use crate::{
    models::TrackInfo,
    util::{errors::ResolverError, source::Source},
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

        Ok(metadata.into())
    }

    fn stream(&self, track: &TrackInfo) -> Result<Input, ResolverError> {
        let request = HttpRequest::new(
            self.get_client(),
            track
                .uri
                .clone()
                .ok_or(ResolverError::MissingRequiredData("uri"))?,
        );

        Ok(Input::from(request))
    }
}
