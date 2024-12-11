use std::{mem::MaybeUninit, pin::Pin, task::{Context, Poll}};

use bytes::BytesMut;
use httparse::{parse_chunk_size, Header, Status};
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::error::ErrorKind;

/// The maximum total size of a request head allowed by the h1 parser
const H1_HTTP_MAX_RESPONSE_HEAD_SIZE: usize = 1024 * 16;

/// The maximum headers qty allowed by the h1 parser
const H1_HTTP_MAX_HEADER_QTY: usize = 128;

const CONNECTION: &str = "connection";
const CLOSE: &str = "close";
const CONTENT_LENGTH: &str = "content-length";
const TRANSFER_ENCODING: &str = "transfer-encoding";
const CHUNKED: &str = "chunked";
const COMMA: u8 = b',';

#[inline(always)]
pub async fn send_request<S: AsyncRead + AsyncWrite + Unpin>(
  stream: &mut S,
  req_buf: &[u8],
  keepalive: bool,
  #[cfg(feature = "timeout")] timeout: Option<std::time::Duration>,
) -> Result<bool, ErrorKind> {
  let inner = async move {
    match stream.write_all(req_buf).await {
      Ok(()) => {}
      Err(_) => return Err(ErrorKind::Write),
    };

    let mut buf = Vec::with_capacity(256);
    
    'read: loop {
      match stream.read_buf(&mut buf).await {
        Ok(0) => return Err(ErrorKind::Read),
        Ok(_) => {
          
          // we gain a little performance here, httparse will take care of initializing the headers
          let mut headers = [MaybeUninit::<Header<'_>>::uninit(); H1_HTTP_MAX_HEADER_QTY];
          let mut res = httparse::Response::new(&mut []);

          let mut config = httparse::ParserConfig::default();
            
          // we set the most permissive config
          config.allow_spaces_after_header_name_in_responses(true);
          config.allow_obsolete_multiline_headers_in_responses(true);
          config.allow_space_before_first_header_name(true);
          config.ignore_invalid_headers_in_responses(true);
         
          // this are for request only
          // config.allow_multiple_spaces_in_request_line_delimiters(true)

          // we limit the parsing to the max size of the response head
          // safety: we cannot overflow as we limit the length to buf.len()
          let slice = unsafe { buf.get_unchecked(..buf.len().min(H1_HTTP_MAX_RESPONSE_HEAD_SIZE)) };
          match config.parse_response_with_uninit_headers(&mut res, slice, &mut headers) {
            Ok(status) => match status {
              httparse::Status::Complete(head_len) => {
                let is_keepalive = 'k: {
                  if !keepalive || res.version != Some(1) {
                    // if disabled keepalive in arguments or server http version is http/1.0 we are not using keepalive
                    break 'k false;
                  } else {
                    for h in res.headers.iter() {
                      if h.name.eq_ignore_ascii_case(CONNECTION) {
                        // if "connection" contains "close" we are not in keepalive
                        break 'k !header_contains(h.value, CLOSE);
                      }
                    }

                    // if no connection header we default to keepalive, as in http/1.1 default
                    break 'k true;
                  }
                };

                let content_length = 'content_length: {
                  for h in res.headers.iter() {
                    if !h.name.eq_ignore_ascii_case(CONTENT_LENGTH) {
                      continue;
                    }

                    let str = match std::str::from_utf8(h.value) {
                      Ok(str) => str,
                      Err(_) => return Err(ErrorKind::Parse),
                    };

                    let len = match str.parse::<u64>() {
                      Ok(v) => v,
                      Err(_) => return Err(ErrorKind::Parse),
                    };

                    break 'content_length Some(len);
                  }

                  None
                };

                match content_length {
                  // identity encoding with content length
                  Some(content_length) => {
                    let to_read = (head_len as u64 + content_length).saturating_sub(buf.len() as u64);

                    if to_read == 0 {
                      return Ok(is_keepalive);
                    }
      
                    match read_and_dispose(stream, to_read).await {
                      Ok(()) => return Ok(is_keepalive),
                      Err(_) => return Err(ErrorKind::ReadBody)
                    }
                  }

                  None => {
                    let is_chunked = 'c: {
                      for h in res.headers.iter() {
                        if h.name.eq_ignore_ascii_case(TRANSFER_ENCODING) {
                          // if "Transfer-Encoding" contains "chunked" item, then the transfer-encoding is chunked 
                          break 'c header_contains(h.value, CHUNKED);
                        }
                      }
                      // if no transfer-encoding header we default to not chunked, as http spec
                      false
                    };

                    // chunked encoding
                    if is_chunked {

                      match consume_chunked_body(stream, &buf[head_len..]).await {
                        Ok(()) => return Ok(is_keepalive),
                        Err(_) => return Err(ErrorKind::ReadBody)
                      }

                    // no chunked encoding nor content-length, consume the response until the end
                    // and dispose the connection, as curl does
                    } else {
                      match read_to_end(stream).await {
                        Ok(()) => return Ok(false),
                        Err(_) => return Err(ErrorKind::ReadBody)
                      }
                    }
                  }
                }
              }

              httparse::Status::Partial => {
                if buf.len() >= H1_HTTP_MAX_RESPONSE_HEAD_SIZE {
                  return Err(ErrorKind::Parse);
                }
                continue 'read;
              }
            },

            Err(_) => return Err(ErrorKind::Parse),
          }
        }

        Err(_) => return Err(ErrorKind::Read),
      }
    }
  };

  #[cfg(not(feature = "timeout"))]
  {
    inner.await
  }

  #[cfg(feature = "timeout")]
  {
    match timeout {
      Some(timeout) => {
        match tokio::time::timeout(timeout, inner).await {
          Ok(res) => res,
          Err(_) => Err(ErrorKind::Timeout),
        }
      } 

      None => inner.await
    }
  }
}

/// Returns true if a list formatted header value contains the needle
/// Eg: "Transfer-Ecoding: chunked,gzip" contains "chunked" and "gzip" 
#[inline(always)]
fn header_contains(v: &[u8], needle: &str) -> bool {
  for item in v.split(|c| *c == COMMA) {
    if item.trim_ascii().eq_ignore_ascii_case(needle.as_bytes()) {
      return true
    }
  }

  false
}


#[inline(always)]
fn read_to_end<R: AsyncRead + Unpin>(r: &mut R) -> ReadToEnd<R> {
  ReadToEnd {
    inner: r,
  }
}

#[pin_project]
struct ReadToEnd<'a, R> {
  #[pin]
  inner: &'a mut R,
}

impl<R: AsyncRead + Unpin> std::future::Future for ReadToEnd<'_, R> {
  type Output = Result<(), std::io::Error>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let mut me = self.project();

    let mut bytes = [MaybeUninit::uninit(); 1024 * 16];
    let mut buf = ReadBuf::uninit(&mut bytes);

    loop {
      match me.inner.as_mut().poll_read(cx, &mut buf) {
        Poll::Pending => return Poll::Pending,
        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
        Poll::Ready(Ok(())) => {
          if buf.filled().is_empty() {
            return Poll::Ready(Ok(())); 
          } else {
            buf.clear();
            continue;
          }
        }
      }
    }
  }
}

#[inline(always)]
fn read_and_dispose<R: AsyncRead + Unpin>(r: &mut R, take: u64) -> ReadAndDispose<R> {
  ReadAndDispose {
    inner: r,
    take,
    n: 0,
  }
}

#[pin_project]
struct ReadAndDispose<'a, R> {
  #[pin]
  inner: &'a mut R,
  take: u64,
  n: u64,
}

impl<R: AsyncRead + Unpin> std::future::Future for ReadAndDispose<'_, R> {
  type Output = Result<(), std::io::Error>; 

  #[inline(always)]
  fn poll(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>
  ) -> Poll<Self::Output> {
    let mut me = self.project();
    loop {
      let mut slice = [MaybeUninit::uninit(); 16 * 1024];
      let mut buf = ReadBuf::uninit(&mut slice);
      match me.inner.as_mut().poll_read(cx, &mut buf) {
        Poll::Pending => return Poll::Pending,
        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
        Poll::Ready(Ok(())) => {
          if buf.filled().is_empty() {
            return Poll::Ready(Err(std::io::ErrorKind::UnexpectedEof.into()));
          } else {
            *me.n += buf.filled().len() as u64;
            
            match (*me.n).cmp(me.take) {
              std::cmp::Ordering::Less => {
                continue;
              }

              std::cmp::Ordering::Equal => {
                return Poll::Ready(Ok(()));
              }

              std::cmp::Ordering::Greater => {
                return Poll::Ready(Err(std::io::ErrorKind::Other.into()));
              }
            }
          }
        }
      }
    }
  }
}

pub async fn consume_chunked_body<R: AsyncRead + Unpin>(stream: &mut R, readed: &[u8]) -> Result<(), std::io::Error> {

  let mut buf = BytesMut::from(readed);
  let mut first = true;

  loop {
    
    if !first || buf.is_empty() {
      match stream.read_buf(&mut buf).await {
        Err(e) => return Err(e), 
        Ok(0) => return Err(std::io::ErrorKind::UnexpectedEof.into()),
        Ok(_) => {}
      }
    }

    first = false;

    match parse_chunk_size(&buf) {
      Ok(status) => {
        match status {
          Status::Complete((consumed, size)) => {
            // last chunk
            if size == 0 {
              let expected = consumed + 2;
              match buf.len().cmp(&expected) {
                std::cmp::Ordering::Equal => {
                  return Ok(())
                }

                std::cmp::Ordering::Less => {
                  let take = expected - buf.len();
                  return read_and_dispose( stream, take as u64).await;
                }

                std::cmp::Ordering::Greater => {
                  return Err(std::io::ErrorKind::Other.into());
                }
              }
            }

            // not last chunk
            let until = consumed as u64 + size + 2; // +2 is for the \r\n at the end of the chunk
            match (buf.len() as u64).cmp(&until) {
              std::cmp::Ordering::Equal => {
                buf = BytesMut::new();
                continue;
              }

              std::cmp::Ordering::Greater => {
                let take = until - buf.len() as u64;
                read_and_dispose(stream, take).await?;
                buf = BytesMut::new();
              }

              std::cmp::Ordering::Less => {
                let _ = buf.split_to(until as usize);
                continue;
              }
            }
          }

          Status::Partial => {
            continue;
          }
        }
      }

      Err(_) => {
        return Err(std::io::ErrorKind::InvalidData.into())
      }
    }
  }
}

