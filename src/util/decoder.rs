//
// Lavalink seems to encode tracks into binary, then serializing it via base64
// And I suck at dealing with binary
// Thanks to @Takase (https://github.com/takase1121) for helping me with this
//

use crate::{constants::TRACK_INFO_VERSIONED, util::errors::Base64DecodeError};
use base64::{Engine, prelude::BASE64_STANDARD};
use byteorder::{BigEndian, ReadBytesExt};
use serde::Serialize;
use std::io::{Cursor, Read};

#[derive(Serialize, Debug)]
pub struct TrackInfo {
    pub flags: u32,
    pub source: String,
    pub identifier: String,
    pub author: String,
    pub length: u64,
    pub is_stream: bool,
    pub position: u64,
    pub title: String,
    pub uri: Option<String>,
    pub artwork_url: Option<String>,
    pub isrc: Option<String>,
    pub version: u32,
}

fn read_string(rdr: &mut Cursor<Vec<u8>>) -> Result<String, Base64DecodeError> {
    let len = rdr.read_u16::<BigEndian>()?;
    let mut buf: Vec<u8> = vec![0; len as usize];
    rdr.read_exact(&mut buf)?;
    Ok(String::from_utf8(buf)?)
}

fn optional_read_string(rdr: &mut Cursor<Vec<u8>>) -> Result<Option<String>, Base64DecodeError> {
    if rdr.read_u8()? != 0 {
        Ok(Some(read_string(rdr)?))
    } else {
        Ok(None)
    }
}

fn parse_v1(mut rdr: Cursor<Vec<u8>>, flags: u32) -> Result<TrackInfo, Base64DecodeError> {
    let title = read_string(&mut rdr)?;
    let author = read_string(&mut rdr)?;
    let length = rdr.read_u64::<BigEndian>()?;
    let identifier = read_string(&mut rdr)?;
    let is_stream = rdr.read_u8()? != 0;
    let source = read_string(&mut rdr)?;
    let position = rdr.read_u64::<BigEndian>()?;

    Ok(TrackInfo {
        flags,
        source,
        identifier,
        author,
        length,
        is_stream,
        position,
        title,
        uri: None,
        artwork_url: None,
        isrc: None,
        version: 1,
    })
}

fn parse_v2(mut rdr: Cursor<Vec<u8>>, flags: u32) -> Result<TrackInfo, Base64DecodeError> {
    let title = read_string(&mut rdr)?;
    let author = read_string(&mut rdr)?;
    let length = rdr.read_u64::<BigEndian>()?;
    let identifier = read_string(&mut rdr)?;
    let is_stream = rdr.read_u8()? != 0;
    let uri = optional_read_string(&mut rdr)?;
    let source = read_string(&mut rdr)?;
    let position = rdr.read_u64::<BigEndian>()?;

    Ok(TrackInfo {
        flags,
        source,
        identifier,
        author,
        length,
        is_stream,
        position,
        title,
        uri,
        artwork_url: None,
        isrc: None,
        version: 2,
    })
}

fn parse_v3(mut rdr: Cursor<Vec<u8>>, flags: u32) -> Result<TrackInfo, Base64DecodeError> {
    let title = read_string(&mut rdr)?;
    let author = read_string(&mut rdr)?;
    let length = rdr.read_u64::<BigEndian>()?;
    let identifier = read_string(&mut rdr)?;
    let is_stream = rdr.read_u8()? != 0;
    let uri = optional_read_string(&mut rdr)?;
    let artwork_url = optional_read_string(&mut rdr)?;
    let isrc = optional_read_string(&mut rdr)?;
    let source = read_string(&mut rdr)?;
    let position = rdr.read_u64::<BigEndian>()?;

    Ok(TrackInfo {
        flags,
        source,
        identifier,
        author,
        length,
        is_stream,
        position,
        title,
        uri,
        artwork_url,
        isrc,
        version: 3,
    })
}

pub fn decode_base64(encoded: &String) -> Result<TrackInfo, Base64DecodeError> {
    let decoded = BASE64_STANDARD.decode(encoded)?;
    let mut rdr = Cursor::new(decoded);
    let value = rdr.read_u32::<BigEndian>()?;
    let flags = (value & 0xC0000000) >> 30;

    let version = if flags & TRACK_INFO_VERSIONED != 0 {
        rdr.read_u8()?
    } else {
        1
    };

    match version {
        1 => Ok(parse_v1(rdr, flags)?),
        2 => Ok(parse_v2(rdr, flags)?),
        3 => Ok(parse_v3(rdr, flags)?),
        _ => Err(Base64DecodeError::UnknownVersion(version)),
    }
}
