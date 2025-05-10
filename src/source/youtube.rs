use std::sync::Arc;

use reqwest::Client;
use rustypipe::{
    client::{ClientType, RustyPipe},
    model::{AudioFormat, UrlTarget},
};
use songbird::{
    input::{Compose, HttpRequest, Input, LiveInput},
    tracks::Track,
};

use crate::{
    models::{ApiPlaylistInfo, ApiTrack, ApiTrackInfo, ApiTrackPlaylist, ApiTrackResult},
    util::{encoder::encode_base64, errors::ResolverError, source::Source},
};

pub struct Youtube {
    client: Client,
    rusty_pipe: RustyPipe,
    client_types: Vec<ClientType>,
}

impl Source for Youtube {
    fn new(client: Option<Client>) -> Self {
        Self {
            client: client.unwrap_or_default(),
            rusty_pipe: RustyPipe::builder()
                .n_http_retries(0)
                .po_token_cache()
                .botguard_bin("./rustypipe/rustypipe-botguard")
                .botguard_snapshot_file("./rustypipe/")
                .storage_dir("./rustypipe/")
                .build()
                .unwrap(),
            client_types: vec![
                ClientType::Desktop,
                ClientType::DesktopMusic,
                ClientType::Ios,
            ],
        }
    }

    fn get_client(&self) -> Client {
        self.client.clone()
    }

    async fn valid_url(&self, url: &str) -> bool {
        self.rusty_pipe.query().resolve_url(url, true).await.is_ok()
    }

    async fn resolve(&self, url: &str) -> Result<ApiTrackResult, ResolverError> {
        let request = self.rusty_pipe.query().resolve_url(url, true).await?;

        let request_url = request.to_url();

        match request {
            UrlTarget::Video { id, .. } => {
                let metadata = self.rusty_pipe.query().video_details(&id).await?;

                let info = ApiTrackInfo {
                    identifier: id.to_owned(),
                    is_seekable: !metadata.is_live,
                    author: metadata.channel.name,
                    // todo: decide on this
                    length: u64::MAX,
                    is_stream: metadata.is_live,
                    position: 0,
                    title: metadata.name,
                    uri: Some(request_url),
                    artwork_url: None,
                    isrc: None,
                    source_name: String::from("youtube"),
                };

                let track = ApiTrack {
                    encoded: encode_base64(&info)?,
                    info,
                    plugin_info: serde_json::Value::Null,
                };

                Ok(ApiTrackResult::Track(track))
            }
            UrlTarget::Channel { .. } => Ok(ApiTrackResult::Empty(None)),
            UrlTarget::Playlist { id } => {
                let mut metadata = self.rusty_pipe.query().playlist(&id).await?;

                let mut playlist = ApiTrackPlaylist {
                    info: ApiPlaylistInfo {
                        name: metadata.name,
                        selected_track: 0,
                    },
                    plugin_info: serde_json::Value::Null,
                    tracks: Vec::new(),
                };

                metadata
                    .videos
                    .extend_pages(self.rusty_pipe.query(), usize::MAX)
                    .await?;

                for video in metadata.videos.items {
                    let url = self
                        .rusty_pipe
                        .query()
                        .resolve_string(&video.id, true)
                        .await?;

                    let info = ApiTrackInfo {
                        identifier: video.id,
                        is_seekable: !video.is_live,
                        author: video
                            .channel
                            .map(|channel| channel.name)
                            .unwrap_or(String::from("Unknown")),
                        length: video.duration.unwrap_or(u32::MAX) as u64,
                        is_stream: video.is_live,
                        position: 0,
                        title: video.name,
                        uri: Some(url.to_url()),
                        artwork_url: video.thumbnail.first().map(|data| data.url.to_owned()),
                        isrc: None,
                        source_name: String::from("youtube"),
                    };

                    let track = ApiTrack {
                        encoded: encode_base64(&info)?,
                        info,
                        plugin_info: serde_json::Value::Null,
                    };

                    playlist.tracks.push(track);
                }

                Ok(ApiTrackResult::Playlist(playlist))
            }
            UrlTarget::Album { .. } => Ok(ApiTrackResult::Empty(None)),
        }
    }

    async fn make_playable(&self, track: ApiTrackInfo) -> Result<Track, ResolverError> {
        let player = self
            .rusty_pipe
            .query()
            .player_from_clients(&track.identifier, &self.client_types)
            .await?;

        let format = player
            .audio_streams
            .iter()
            .filter(|stream| stream.format == AudioFormat::Webm)
            .reduce(|prev, current| {
                if prev.bitrate > current.bitrate {
                    prev
                } else {
                    current
                }
            })
            .ok_or(ResolverError::MissingRequiredData("Audio Stream"))?;

        let mut request = HttpRequest::new(self.get_client(), format.url.clone());

        let stream = request.create_async().await?;
        let input = Input::Live(LiveInput::Raw(stream), None);

        Ok(Track::new_with_data(input, Arc::new(track)))
    }
}
