use crate::{
    constants::TRACK_INFO_VERSIONED, models::RawTrackInfo, util::errors::Base64EncodeError,
};
use base64::{Engine, prelude::BASE64_STANDARD};
use byteorder::{BigEndian, WriteBytesExt};
use std::io::{Cursor, Write};

fn write_string(wtr: &mut Cursor<Vec<u8>>, value: &str) -> Result<(), Base64EncodeError> {
    let len = value.len() as u16;
    wtr.write_u16::<BigEndian>(len)?;
    wtr.write_all(value.as_bytes())?;
    Ok(())
}

fn write_optional_string(
    wtr: &mut Cursor<Vec<u8>>,
    value: &Option<String>,
) -> Result<(), Base64EncodeError> {
    match value {
        Some(v) => {
            wtr.write_u8(1)?;
            write_string(wtr, v)?;
        }
        None => {
            wtr.write_u8(0)?;
        }
    }
    Ok(())
}

fn encode_v1(wtr: &mut Cursor<Vec<u8>>, track: &RawTrackInfo) -> Result<(), Base64EncodeError> {
    write_string(wtr, &track.title)?;
    write_string(wtr, &track.author)?;
    wtr.write_u64::<BigEndian>(track.length)?;
    write_string(wtr, &track.identifier)?;
    wtr.write_u8(if track.is_stream { 1 } else { 0 })?;
    write_string(wtr, &track.source)?;
    wtr.write_u64::<BigEndian>(track.position)?;
    Ok(())
}

fn encode_v2(wtr: &mut Cursor<Vec<u8>>, track: &RawTrackInfo) -> Result<(), Base64EncodeError> {
    write_string(wtr, &track.title)?;
    write_string(wtr, &track.author)?;
    wtr.write_u64::<BigEndian>(track.length)?;
    write_string(wtr, &track.identifier)?;
    wtr.write_u8(if track.is_stream { 1 } else { 0 })?;
    write_optional_string(wtr, &track.uri)?;
    write_string(wtr, &track.source)?;
    wtr.write_u64::<BigEndian>(track.position)?;
    Ok(())
}

fn encode_v3(wtr: &mut Cursor<Vec<u8>>, track: &RawTrackInfo) -> Result<(), Base64EncodeError> {
    write_string(wtr, &track.title)?;
    write_string(wtr, &track.author)?;
    wtr.write_u64::<BigEndian>(track.length)?;
    write_string(wtr, &track.identifier)?;
    wtr.write_u8(if track.is_stream { 1 } else { 0 })?;
    write_optional_string(wtr, &track.uri)?;
    write_optional_string(wtr, &track.artwork_url)?;
    write_optional_string(wtr, &track.isrc)?;
    write_string(wtr, &track.source)?;
    wtr.write_u64::<BigEndian>(track.position)?;
    Ok(())
}

// we will encode using v3 only in this library
pub fn encode_base64(track: &RawTrackInfo) -> Result<String, Base64EncodeError> {
    let mut buffer = Vec::new();
    let mut writer = Cursor::new(buffer);

    let mut flags = track.flags;

    if track.version >= 2 {
        flags |= TRACK_INFO_VERSIONED;
    }

    let value = (flags << 30) | 0x3FFFFFFF;
    writer.write_u32::<BigEndian>(value)?;

    if flags & TRACK_INFO_VERSIONED != 0 {
        writer.write_u8(track.version as u8)?;
    }

    match track.version {
        1 => encode_v1(&mut writer, track)?,
        2 => encode_v2(&mut writer, track)?,
        3 => encode_v3(&mut writer, track)?,
        _ => return Err(Base64EncodeError::UnknownVersion(track.version)),
    }

    buffer = writer.into_inner();

    let encoded = BASE64_STANDARD.encode(&buffer);

    Ok(encoded)
}
