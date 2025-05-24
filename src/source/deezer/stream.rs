use super::{CHUNK_SIZE, SECRET_IV};
use crate::util::seek::SeekableSource;
use async_trait::async_trait;
use blowfish::Blowfish;
use cbc::cipher::{KeyIvInit, block_padding::NoPadding};
use cbc::{Decryptor, cipher::BlockDecryptMut};
use songbird::input::{AudioStream, AudioStreamError, Compose, HttpRequest};
use std::cmp::min;
use std::io::{Error as IoError, ErrorKind, Read, Result as IoResult, Seek, SeekFrom};
use symphonia::core::io::MediaSource;
use tokio::task::block_in_place;

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
        let request = self.request.create_async().await?;

        let seekable = SeekableSource::new(request.input);
        let hint = request.hint;

        Ok(AudioStream {
            input: Box::new(DeezerMediaSource::new(Box::new(seekable), self.key))
                as Box<dyn MediaSource>,
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
    position: usize
}

impl Read for DeezerMediaSource {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let mut total_read = 0;

        while total_read < CHUNK_SIZE {
            let bytes_read = block_in_place(|| self.source.read(&mut self.buffer[total_read..]))?;

            if bytes_read == 0 {
                break;
            }

            total_read += bytes_read;
        }

        let current_chunk = self.position / CHUNK_SIZE;

        let end = min(buf.len(), total_read);

        if current_chunk % 3 > 0 || total_read < CHUNK_SIZE {
            buf[..end].copy_from_slice(&self.buffer[..end]);
        } else {
            let decryptor: Decryptor<Blowfish> = Decryptor::new_from_slices(&self.key, &SECRET_IV)
                .map_err(|error| IoError::new(ErrorKind::Unsupported, error))?;

            let decrypted = decryptor
                .decrypt_padded_mut::<NoPadding>(&mut self.buffer[..total_read])
                .map_err(|error| IoError::new(ErrorKind::InvalidInput, error.to_string()))?;

            buf[..end].copy_from_slice(&decrypted[..end]);
        }

        self.position += total_read;

        Ok(end)
    }
}

impl Seek for DeezerMediaSource {
    fn seek(&mut self, position: SeekFrom) -> IoResult<u64> {
        self.source.seek(position)
    }
}

impl MediaSource for DeezerMediaSource {
    fn is_seekable(&self) -> bool {
        false
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
            position: 0,
        }
    }
}

impl DeezerHttpStream {
    pub fn new(request: HttpRequest, key: [u8; 16]) -> Self {
        Self { request, key }
    }
}
