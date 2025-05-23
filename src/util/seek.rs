use songbird::input::AudioStream;
use std::{
    cmp::min,
    io::{Error as IoError, ErrorKind, Read, Result as IoResult, Seek, SeekFrom},
};
use symphonia::core::{io::MediaSource, probe::Hint};
use tokio::task::block_in_place;

static CHUNK_SIZE: usize = 128;

pub struct SeekableSource {
    source: Box<dyn MediaSource>,
    seekable: bool,
    position: usize,
    downloaded: Vec<u8>,
    downloaded_bytes: usize,
    content_length: Option<usize>,
}

impl Read for SeekableSource {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        if !self.seekable {
            let bytes_read = block_in_place(|| self.source.read(buf))?;
            self.position += bytes_read;
            self.downloaded_bytes += bytes_read;
            return Ok(self.downloaded_bytes);
        }

        if self.position < self.downloaded_bytes {
            let mut read_up_to = self.position + min(buf.len(), CHUNK_SIZE);

            if read_up_to > self.downloaded_bytes {
                read_up_to = self.downloaded_bytes;
            }

            let bytes = &self.downloaded[self.position..read_up_to];
            let bytes_read = bytes.len();

            buf[0..bytes_read].copy_from_slice(bytes);

            self.position += bytes_read;

            return Ok(bytes_read);
        }

        let bytes_read = block_in_place(|| self.source.read(buf))?;

        if bytes_read == 0 {
            return Ok(bytes_read);
        }

        self.position += bytes_read;
        self.downloaded_bytes += bytes_read;

        self.downloaded.extend(buf[0..bytes_read].iter());

        Ok(bytes_read)
    }
}

impl Seek for SeekableSource {
    fn seek(&mut self, position: SeekFrom) -> IoResult<u64> {
        let new_position = match position {
            SeekFrom::Start(n) => n as usize,
            SeekFrom::Current(offset) => {
                let pos = self.position as i64 + offset;

                if pos < 0 {
                    return Err(IoError::new(ErrorKind::InvalidInput, "Negative seek"));
                }

                pos as usize
            }
            SeekFrom::End(offset) => {
                let length = self
                    .content_length
                    .ok_or_else(|| IoError::new(ErrorKind::Unsupported, "Length unknown"))?;

                let pos = length as i64 + offset;

                if pos < 0 {
                    return Err(IoError::new(ErrorKind::InvalidInput, "Negative seek"));
                }

                pos as usize
            }
        };

        self.position = new_position.min(self.content_length.unwrap_or(usize::MAX));

        Ok(self.position as u64)
    }
}

impl MediaSource for SeekableSource {
    fn is_seekable(&self) -> bool {
        self.content_length.is_some()
    }

    fn byte_len(&self) -> Option<u64> {
        self.content_length.map(|length| length as u64)
    }
}

impl SeekableSource {
    pub fn new(source: Box<dyn MediaSource>) -> Self {
        let content_length = block_in_place(|| source.byte_len().map(|size| size as usize));

        let downloaded = {
            if let Some(length) = content_length {
                Vec::with_capacity(length + 1000)
            } else {
                Vec::new()
            }
        };

        Self {
            source,
            downloaded,
            content_length,
            position: 0,
            downloaded_bytes: 0,
            seekable: content_length.is_some(),
        }
    }

    pub fn into_audio_stream(self, hint: Option<Hint>) -> AudioStream<Box<dyn MediaSource>> {
        AudioStream {
            input: Box::new(self) as Box<dyn MediaSource>,
            hint,
        }
    }
}
