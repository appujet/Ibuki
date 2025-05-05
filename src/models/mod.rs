use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

//
// Lavalink Types (To be refactored in future)
//

fn str_to_u64<'de, T, D>(de: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    String::deserialize(de)?
        .parse()
        .map_err(serde::de::Error::custom)
}

fn u64_to_str<S>(num: &u64, se: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    se.serialize_str(num.to_string().as_str())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Common,
    Suspicious,
    Fault,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadType {
    Track,
    Playlist,
    Search,
    Empty,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "loadType", content = "data")]
pub enum DataType {
    Track(Track),
    Playlist(TrackPlaylist),
    Search(Vec<Track>),
    Error(TrackLoadException),
    Empty(Option<Value>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistInfo {
    pub name: String,
    pub selected_track: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackPlaylist {
    pub info: PlaylistInfo,
    pub plugin_info: Value,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackLoadException {
    pub message: String,
    pub severity: Severity,
    pub cause: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceData {
    pub token: String,
    pub endpoint: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ping: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub time: u64,
    pub position: u32,
    pub connected: bool,
    pub ping: Option<i32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    #[serde(deserialize_with = "str_to_u64", serialize_with = "u64_to_str")]
    pub guild_id: u64,
    pub track: Option<Track>,
    pub volume: u32,
    pub paused: bool,
    pub state: PlayerState,
    pub voice: VoiceData,
    pub filters: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub identifier: String,
    pub is_seekable: bool,
    pub author: String,
    pub length: u64,
    pub is_stream: bool,
    pub position: u64,
    pub title: String,
    pub uri: Option<String>,
    pub artwork_url: Option<String>,
    pub isrc: Option<String>,
    pub source_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub encoded: String,
    pub info: TrackInfo,
    pub plugin_info: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exception {
    #[serde(deserialize_with = "str_to_u64", serialize_with = "u64_to_str")]
    pub guild_id: u64,
    pub message: Option<String>,
    pub severity: String,
    pub cause: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackStart {
    #[serde(deserialize_with = "str_to_u64", serialize_with = "u64_to_str")]
    pub guild_id: u64,
    pub track: Track,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackEnd {
    #[serde(deserialize_with = "str_to_u64", serialize_with = "u64_to_str")]
    pub guild_id: u64,
    pub track: Track,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackException {
    #[serde(deserialize_with = "str_to_u64", serialize_with = "u64_to_str")]
    pub guild_id: u64,
    pub track: Track,
    pub exception: Exception,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackStuck {
    #[serde(deserialize_with = "str_to_u64", serialize_with = "u64_to_str")]
    pub guild_id: u64,
    pub track: Track,
    pub threshold_ms: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketClosed {
    #[serde(deserialize_with = "str_to_u64", serialize_with = "u64_to_str")]
    pub guild_id: u64,
    pub code: usize,
    pub reason: String,
    pub by_remote: bool,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlayerEvents {
    TrackStartEvent(TrackStart),
    TrackEndEvent(TrackEnd),
    TrackExceptionEvent(TrackException),
    TrackStuckEvent(TrackStuck),
    WebSocketClosedEvent(WebSocketClosed),
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePlayerTrack {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoded: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track: Option<UpdatePlayerTrack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<VoiceData>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct FrameStats {
    pub sent: u64,
    pub nulled: u32,
    pub deficit: i32,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cpu {
    pub cores: u32,
    pub system_load: f64,
    pub lavalink_load: f64,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Memory {
    pub free: u64,
    pub used: u64,
    pub allocated: u64,
    pub reservable: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ready {
    pub resumed: bool,
    pub session_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerUpdate {
    #[serde(deserialize_with = "str_to_u64", serialize_with = "u64_to_str")]
    pub guild_id: u64,
    pub state: PlayerState,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub players: u32,
    pub playing_players: u32,
    pub uptime: u64,
    pub memory: Memory,
    pub cpu: Cpu,
    pub frame_stats: Option<FrameStats>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op")]
#[serde(rename_all = "camelCase")]
pub enum NodeMessage {
    Ready(Ready),
    PlayerUpdate(PlayerUpdate),
    Stats(Stats),
    Event(PlayerEvents),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    resuming: bool,
    timeout: u32,
}
