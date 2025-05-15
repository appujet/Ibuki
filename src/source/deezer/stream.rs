use super::{CHUNK_SIZE, SECRET_IV};

use async_trait::async_trait;
use blowfish::Blowfish;
use cbc::cipher::{KeyIvInit, block_padding::NoPadding};
use cbc::{Decryptor, cipher::BlockDecryptMut};
use songbird::input::{AudioStream, AudioStreamError, Compose, HttpRequest};
use std::cmp::min;
use std::io::{Read, Seek, SeekFrom};
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
            input: Box::new(DeezerMediaSource::new(
                stream.input,
                Decryptor::new_from_slices(&self.key, &SECRET_IV).unwrap(),
            )) as Box<dyn MediaSource>,
            hint,
        })
    }

    fn should_create_async(&self) -> bool {
        self.request.should_create_async()
    }
}

pub struct DeezerMediaSource {
    source: Box<dyn MediaSource>,
    buffer: [u8; CHUNK_SIZE],
    buffer_len: usize,
    current_chunk: usize,
    decryptor: Decryptor<Blowfish>,
    decrypted: Vec<u8>,
}

impl Read for DeezerMediaSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
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
        } else if let Ok(decrypted) = self
            .decryptor
            .clone()
            .decrypt_padded_mut::<NoPadding>(&mut self.buffer[..self.buffer_len])
        {
            self.decrypted.extend(decrypted);
        } else {
            tracing::warn!("Failed to decrypt a chunk");
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
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
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
    pub fn new(source: Box<dyn MediaSource>, decryptor: Decryptor<Blowfish>) -> Self {
        Self {
            source,
            buffer: [0; CHUNK_SIZE],
            buffer_len: CHUNK_SIZE,
            current_chunk: 0,
            decryptor,
            decrypted: Vec::new(),
        }
    }
}
