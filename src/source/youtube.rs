use std::sync::Arc;

use bytesize::ByteSize;
use reqwest::Client;
use rustypipe::{
    client::{ClientType, RustyPipe},
    model::{UrlTarget, VideoPlayer, YouTubeItem},
    param::search_filter::{ItemType, SearchFilter},
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
    video_itags: Vec<u32>,
    audio_itags: Vec<u32>,
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
                ClientType::Mobile,
            ],
            video_itags: vec![18, 22, 37, 44, 45, 46],
            audio_itags: vec![140, 141, 171, 250, 251],
        }
    }

    fn get_name(&self) -> &'static str {
        "youtube"
    }

    fn get_client(&self) -> Client {
        self.client.clone()
    }

    async fn valid_url(&self, url: &str) -> bool {
        self.rusty_pipe.query().resolve_url(url, true).await.is_ok()
    }

    async fn try_search(&self, query: &str) -> bool {
        query.starts_with("ytsearch") || query.starts_with("ytmsearch")
    }

    async fn search(&self, query: &str) -> Result<ApiTrackResult, ResolverError> {
        let term = query
            .strip_prefix("ytsearch")
            .or(query.strip_prefix("ytmsearch"))
            .ok_or(ResolverError::InputNotSupported)?;

        let (prefix, _) = query.split_at(query.len() - term.len());

        match prefix {
            "ytsearch" => {
                let filter = SearchFilter::new().item_type(ItemType::Video);

                let results = self
                    .rusty_pipe
                    .query()
                    .search_filter::<YouTubeItem, _>(query, &filter)
                    .await?;

                let mut tracks = Vec::new();

                for result in results.items.items {
                    match result {
                        YouTubeItem::Video(video) => {
                            let info = ApiTrackInfo {
                                identifier: video.id.to_owned(),
                                is_seekable: !video.is_live,
                                author: video
                                    .channel
                                    .map(|channel| channel.name)
                                    .unwrap_or(String::from("Unknown")),
                                length: video.duration.unwrap_or(u32::MAX) as u64,
                                is_stream: video.is_live,
                                position: 0,
                                title: video.name,
                                uri: Some(format!("https://www.youtube.com/watch?{}", video.id)),
                                artwork_url: video
                                    .thumbnail
                                    .first()
                                    .map(|data| data.url.to_owned()),
                                isrc: None,
                                source_name: self.get_name().into(),
                            };

                            let track = ApiTrack {
                                encoded: encode_base64(&info)?,
                                info,
                                plugin_info: serde_json::Value::Null,
                            };

                            tracks.push(track);
                        }
                        _ => return Err(ResolverError::MissingRequiredData("Video Item")),
                    }
                }

                Ok(ApiTrackResult::Search(tracks))
            }

            "ytmsearch" => {
                let results = self.rusty_pipe.query().music_search_videos(query).await?;

                let mut tracks = Vec::new();

                for result in results.items.items {
                    let info = ApiTrackInfo {
                        identifier: result.id.to_owned(),
                        is_seekable: true,
                        author: result
                            .artists
                            .first()
                            .map(|artist| artist.name.to_owned())
                            .unwrap_or(String::from("Unknown")),
                        length: result.duration.unwrap_or(0) as u64,
                        is_stream: result.duration.map(|_| true).unwrap_or(false),
                        position: 0,
                        title: result.name,
                        uri: Some(format!("https://music.youtube.com/watch?v={}", result.id)),
                        artwork_url: result.cover.first().map(|data| data.url.to_owned()),
                        isrc: None,
                        source_name: self.get_name().into(),
                    };

                    let track = ApiTrack {
                        encoded: encode_base64(&info)?,
                        info,
                        plugin_info: serde_json::Value::Null,
                    };

                    tracks.push(track);
                }

                Ok(ApiTrackResult::Search(tracks))
            }
            _ => Err(ResolverError::InputNotSupported),
        }
    }

    async fn resolve(&self, url: &str) -> Result<ApiTrackResult, ResolverError> {
        let request = self.rusty_pipe.query().resolve_url(url, true).await?;

        let request_url = request.to_url();

        match request {
            UrlTarget::Video { id, .. } => {
                let player = self.rusty_pipe.query().player(&id).await?;

                let metadata = player.details;

                let info = ApiTrackInfo {
                    identifier: id.to_owned(),
                    is_seekable: !metadata.is_live,
                    author: metadata.channel_name.unwrap_or(String::from("Unknown")),
                    length: metadata.duration as u64,
                    is_stream: metadata.is_live,
                    position: 0,
                    title: metadata.name.unwrap_or(String::from("Unknown")),
                    uri: Some(request_url),
                    artwork_url: metadata.thumbnail.first().map(|data| data.url.to_owned()),
                    isrc: None,
                    source_name: self.get_name().into(),
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
                        source_name: self.get_name().into(),
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

    async fn make_playable(&self, track: ApiTrack) -> Result<Track, ResolverError> {
        let player = {
            let mut result: Option<VideoPlayer> = None;

            for client in &self.client_types {
                let video = self
                    .rusty_pipe
                    .query()
                    .player_from_client(&track.info.identifier, *client)
                    .await;

                let client_name = format!("Client [{}]", self.readable_client_type(client));

                match video {
                    Ok(video) => {
                        if video.audio_streams.is_empty() && video.video_streams.is_empty() {
                            tracing::warn!(
                                "{} failed to get results due to: No streams available",
                                client_name,
                            );
                            continue;
                        }

                        tracing::info!(
                            "{} got results! Formats Count => [Audio: {}]  [Video: {}]",
                            client_name,
                            video.audio_streams.len(),
                            video.video_streams.len()
                        );

                        let _ = result.insert(video);

                        break;
                    }
                    Err(err) => {
                        tracing::warn!("{} failed to get results due to: {:?}", client_name, err);
                    }
                }
            }

            result.ok_or(ResolverError::MissingRequiredData(
                "Failed to resolve an Api Track",
            ))?
        };

        let audio = player
            .audio_streams
            .iter()
            .filter(|stream| self.audio_itags.contains(&stream.itag))
            .reduce(|prev, current| {
                if prev.bitrate > current.bitrate {
                    prev
                } else {
                    current
                }
            });

        if let Some(stream) = audio {
            tracing::info!(
                "Picked [{}] [{} ({}/s)] for the playback",
                stream.itag,
                stream.mime,
                ByteSize::b(stream.bitrate as u64).display().iec_short()
            );
        }

        let video = player
            .video_streams
            .iter()
            .filter(|stream| self.video_itags.contains(&stream.itag))
            .reduce(|prev, current| {
                if prev.bitrate > current.bitrate {
                    prev
                } else {
                    current
                }
            });

        if let Some(stream) = audio
            && audio.is_none()
        {
            tracing::info!(
                "Picked [{}] [{} ({}/s)] for the playback",
                stream.itag,
                stream.mime,
                ByteSize::b(stream.bitrate as u64).display().iec_short()
            );
        }

        let mut url = audio.map(|stream| &stream.url);

        if url.is_none() {
            url = video.map(|stream| &stream.url);
        }

        let mut request = HttpRequest::new(
            self.get_client(),
            url.ok_or(ResolverError::MissingRequiredData("Stream to Play"))?
                .clone(),
        );

        let stream = request.create_async().await?;
        let input = Input::Live(LiveInput::Raw(stream), None);

        Ok(Track::new_with_data(input, Arc::new(track)))
    }
}

impl Youtube {
    pub fn readable_client_type(&self, client: &ClientType) -> &'static str {
        match client {
            ClientType::Desktop => "Desktop",
            ClientType::DesktopMusic => "Desktop Music",
            ClientType::Mobile => "Mobile",
            ClientType::Tv => "TV",
            ClientType::Android => "Android",
            ClientType::Ios => "IOS",
            _ => "Unknown",
        }
    }
}
