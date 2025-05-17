/*
 * This is from rseek (https://github.com/sam0x17/rseek/tree/main)
 * Made to fit this program needs
 * The original implementation from the original code gives me problems
 * So I decided to fork it and put it here
 */
use super::errors::SeekableInitError;
use async_trait::async_trait;
use bytes::Bytes;
use futures::TryStreamExt;
use reqwest::header::{ACCEPT_RANGES, CONTENT_LENGTH, CONTENT_TYPE, RETRY_AFTER};
use reqwest::{Client, Response};
use songbird::input::{AsyncMediaSource, AudioStreamError};
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;
use symphonia::core::probe::Hint;
use tokio::io::{AsyncRead, AsyncSeek, ReadBuf, SeekFrom};
use tokio_util::io::StreamReader;

type Reader =
    StreamReader<Pin<Box<dyn futures::Stream<Item = Result<Bytes, IoError>> + Send>>, Bytes>;

pub struct Seekable {
    pub position: u64,
    pub url: String,
    pub length: Option<u64>,
    pub hint: Option<Hint>,
    client: Client,
    reader: Option<Reader>,
    fetcher: Option<Pin<Box<dyn futures::Future<Output = IoResult<Response>> + Send>>>,
    resumable: Arc<AtomicBool>,
}

impl Unpin for Seekable {}

unsafe impl Sync for Seekable {}

impl Seekable {
    pub fn new(client: Client, url: String) -> Self {
        Seekable {
            client,
            url,
            position: 0,
            hint: None,
            length: None,
            reader: None,
            fetcher: None,
            resumable: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn init_seekable(&mut self) -> Result<(), SeekableInitError> {
        let builder = self.client.get(&self.url);

        let response = builder
            .send()
            .await
            .map_err(|error| SeekableInitError::FailedGet(error.to_string()))?;

        if !response.status().is_success() {
            return Err(SeekableInitError::FailedStatusCode(format!(
                "Seekable Init: {}",
                response.status()
            )));
        }

        if let Some(t) = response.headers().get(RETRY_AFTER) {
            t.to_str()
                .map_err(|_| {
                    SeekableInitError::InvalidRetryHeader(String::from(
                        "Retry-after field contained non-ASCII data.",
                    ))
                })
                .and_then(|str_text| {
                    str_text.parse().map_err(|_| {
                        SeekableInitError::InvalidRetryHeader(String::from(
                            "Retry-after field was non-numeric.",
                        ))
                    })
                })
                .and_then(|t| Err(SeekableInitError::RetryIn(Duration::from_secs(t).as_secs())))
        } else {
            let headers = response.headers();

            let hint = headers
                .get(CONTENT_TYPE)
                .and_then(|val| val.to_str().ok())
                .map(|val| {
                    let mut out = Hint::default();
                    out.mime_type(val);
                    out
                });

            self.hint = hint;

            let length = headers
                .get(CONTENT_LENGTH)
                .and_then(|val| val.to_str().ok())
                .and_then(|val| val.parse::<u64>().ok());

            self.length = length;

            let resume = headers
                .get(ACCEPT_RANGES)
                .and_then(|a| a.to_str().ok())
                .filter(|a| *a == "bytes");

            if resume.is_some() {
                self.resumable.swap(true, Ordering::Relaxed);
            }

            Ok(())
        }
    }

    pub fn fetch_next_poll(&mut self, pos: u64) {
        self.reader = None;

        let mut builder = self.client.get(&self.url);

        if let Some(size) = self.length {
            let end = size.saturating_sub(1);

            builder = builder.header("Range", format!("bytes={pos}-{end}"));
        } else {
            builder = builder.header("Range", format!("bytes={pos}-"));
        }

        let future = async move {
            builder
                .send()
                .await
                .map_err(|e| IoError::other(e.to_string()))
        };

        self.fetcher = Some(Box::pin(future));
    }
}

impl AsyncRead for Seekable {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        let this = unsafe { Pin::get_unchecked_mut(self) };

        if let Some(sz) = this.length {
            if this.position >= sz {
                return Poll::Ready(Err(IoError::new(ErrorKind::UnexpectedEof, "EOF reached")));
            }
        }

        if let Some(reader) = &mut this.reader {
            let before = buf.filled().len();

            let res = Pin::new(reader).poll_read(cx, buf);

            if let Poll::Ready(Ok(())) = &res {
                this.position += (buf.filled().len() - before) as u64;
            }

            return res;
        }

        if let Some(future) = &mut this.fetcher {
            match future.as_mut().poll(cx) {
                Poll::Ready(Ok(resp)) => {
                    let stream = resp
                        .bytes_stream()
                        .map_err(|e| IoError::other(e.to_string()));

                    this.reader = Some(StreamReader::new(Box::pin(stream)));

                    this.fetcher = None;

                    let pinned = unsafe { Pin::new_unchecked(this) };

                    return AsyncRead::poll_read(pinned, cx, buf);
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            }
        }

        Poll::Ready(Err(IoError::new(ErrorKind::UnexpectedEof, "Stream closed")))
    }
}

impl AsyncSeek for Seekable {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> IoResult<()> {
        let this = self.get_mut();

        let new_pos = match position {
            SeekFrom::Start(n) => n,
            SeekFrom::Current(off) => {
                let tmp = this.position as i64 + off;

                if tmp < 0 {
                    return Err(IoError::new(ErrorKind::InvalidInput, "Negative seek"));
                }

                tmp as u64
            }
            SeekFrom::End(off) => {
                let sz = this
                    .length
                    .ok_or_else(|| IoError::new(ErrorKind::Unsupported, "Length unknown"))?;

                let tmp = sz as i64 + off;

                if tmp < 0 {
                    return Err(IoError::new(ErrorKind::InvalidInput, "Negative seek"));
                }

                tmp as u64
            }
        };

        this.position = new_pos.min(this.length.unwrap_or(u64::MAX));

        this.fetcher = None;

        this.fetch_next_poll(this.position);

        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<u64>> {
        let this = self.get_mut();

        Poll::Ready(Ok(this.position))
    }
}

#[async_trait]
impl AsyncMediaSource for Seekable {
    fn is_seekable(&self) -> bool {
        true
    }

    async fn byte_len(&self) -> Option<u64> {
        self.length
    }

    async fn try_resume(
        &mut self,
        offset: u64,
    ) -> Result<Box<dyn AsyncMediaSource>, AudioStreamError> {
        if !self.resumable.load(Ordering::Relaxed) {
            return Err(AudioStreamError::Unsupported);
        }

        let mut seekable = Box::new(Seekable::new(self.client.clone(), self.url.clone()));

        seekable.init_seekable().await.map_err(|err| {
            let msg: Box<dyn std::error::Error + Send + Sync + 'static> =
                format!("Failed with init seekable error: {err}").into();

            AudioStreamError::Fail(msg)
        })?;

        seekable.fetch_next_poll(offset);

        Ok(seekable)
    }
}
