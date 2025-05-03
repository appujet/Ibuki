use crate::{
    constants::TRACK_INFO_VERSIONED, models::RawTrackInfo, util::errors::Base64EncodeError,
};
use base64::{Engine, prelude::BASE64_STANDARD};
use byteorder::{BigEndian, WriteBytesExt};
use std::io::{Cursor, Write};

fn write_string(wtr: &mut Cursor<Vec<u8>>, message: &str) -> Result<(), Base64EncodeError> {
    wtr.write_u16::<BigEndian>(message.len() as u16)?;
    wtr.write_all(message.as_bytes())?;
    Ok(())
}

fn optional_write_string(
    wtr: &mut Cursor<Vec<u8>>,
    opt: &Option<String>,
) -> Result<(), Base64EncodeError> {
    match opt {
        Some(s) => {
            wtr.write_u8(1)?;
            write_string(wtr, s)?;
        }
        None => {
            wtr.write_u8(0)?;
        }
    }
    Ok(())
}

/**
 * Unfortunately this isnt cross compatible with lavalink for some reason
 */
pub fn encode_base64(track_info: &RawTrackInfo) -> Result<String, Base64EncodeError> {
    if track_info.version > 3 || track_info.version == 0 {
        return Err(Base64EncodeError::UnknownVersion(track_info.version));
    }

    let mut wtr = Cursor::new(Vec::new());

    wtr.write_u32::<BigEndian>(((track_info.flags & 0x3) << 30) | TRACK_INFO_VERSIONED)?;

    wtr.write_u8(3)?;

    write_string(&mut wtr, &track_info.title)?;
    write_string(&mut wtr, &track_info.author)?;
    wtr.write_u64::<BigEndian>(track_info.length)?;
    write_string(&mut wtr, &track_info.identifier)?;
    wtr.write_u8(if track_info.is_stream { 1 } else { 0 })?;

    optional_write_string(&mut wtr, &track_info.uri)?;
    optional_write_string(&mut wtr, &track_info.artwork_url)?;
    optional_write_string(&mut wtr, &track_info.isrc)?;

    write_string(&mut wtr, &track_info.source)?;

    wtr.write_u64::<BigEndian>(track_info.position)?;

    Ok(BASE64_STANDARD.encode(wtr.into_inner()))
}
