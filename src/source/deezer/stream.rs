use super::{CHUNK_SIZE, SECRET_IV};

use async_trait::async_trait;
use blowfish::Blowfish;
use cbc::cipher::{KeyIvInit, block_padding::NoPadding};
use cbc::{Decryptor, cipher::BlockDecryptMut};
use songbird::input::{AudioStream, AudioStreamError, Compose, HttpRequest};
use std::cmp::min;
use std::io::{Error as IoError, ErrorKind, Read, Result as IoResult, Seek, SeekFrom};
use symphonia::core::io::MediaSource;

pub struct DeezerHttpStream {
    request: HttpRequest,
    key: [u8; 16],
}

#[async_trait]
impl Compose for DeezerHttpStream {
    fn create(&mut self) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        self.request.create()
    }

    async fn create_async(
        &mut self,
    ) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        let stream = self.request.create_async().await?;

        let hint = stream.hint;

        Ok(AudioStream {
            input: Box::new(DeezerMediaSource::new(stream.input, self.key)) as Box<dyn MediaSource>,
            hint,
        })
    }

    fn should_create_async(&self) -> bool {
        self.request.should_create_async()
    }
}

pub struct DeezerMediaSource {
    source: Box<dyn MediaSource>,
    key: [u8; 16],
    buffer: [u8; CHUNK_SIZE],
    buffer_len: usize,
    current_chunk: usize,
    decrypted: Vec<u8>,
}

impl Read for DeezerMediaSource {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        while self.buffer_len < CHUNK_SIZE {
            // reads the source by CHUNK_SIZE, then is inserted into self.buffer, overwriting the old one
            let bytes_read = self.source.read(&mut self.buffer[self.buffer_len..])?;

            if bytes_read == 0 {
                break;
            }

            self.buffer_len += bytes_read;
        }

        if self.current_chunk % 3 > 0 || self.buffer_len < CHUNK_SIZE {
            self.decrypted.extend(&self.buffer[..self.buffer_len]);
        } else {
            let decryptor: Decryptor<Blowfish> = Decryptor::new_from_slices(&self.key, &SECRET_IV)
                .map_err(|error| IoError::new(ErrorKind::Unsupported, error))?;

            let decrypted = decryptor
                .decrypt_padded_mut::<NoPadding>(&mut self.buffer[..self.buffer_len])
                .map_err(|error| IoError::new(ErrorKind::InvalidInput, error.to_string()))?;

            self.decrypted.extend(decrypted);
        }

        // reset buffer_len so the next write would write over the top of the old one
        self.buffer_len = 0;
        // advance chunk
        self.current_chunk += 1;

        let end = min(buf.len(), self.decrypted.len());

        let drain = self.decrypted.drain(0..end);

        let drain_len = drain.len();

        buf[0..drain_len].copy_from_slice(drain.as_ref());

        Ok(drain_len)
    }
}

impl Seek for DeezerMediaSource {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        self.source.seek(pos)
    }
}

impl MediaSource for DeezerMediaSource {
    fn is_seekable(&self) -> bool {
        self.source.is_seekable()
    }

    fn byte_len(&self) -> Option<u64> {
        self.source.byte_len()
    }
}

impl DeezerMediaSource {
    pub fn new(source: Box<dyn MediaSource>, key: [u8; 16]) -> Self {
        Self {
            source,
            key,
            buffer: [0; CHUNK_SIZE],
            buffer_len: CHUNK_SIZE,
            current_chunk: 0,
            decrypted: Vec::new(),
        }
    }
}

impl DeezerHttpStream {
    pub fn new(request: HttpRequest, key: [u8; 16]) -> Self {
        Self { request, key }
    }
}
