use std::{mem::MaybeUninit, pin::Pin, task::{Context, Poll}};

use bytes::BytesMut;
use httparse::{parse_chunk_size, Status};
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

#[inline(always)]
pub async fn send_request<S: AsyncRead + AsyncWrite + Unpin>(
  stream: &mut S,
  req_buf: &[u8],
  keepalive: bool,
) -> Result<bool, ()> {
  match stream.write_all(req_buf).await {
    Ok(()) => {}
    Err(_) => return Err(()),
  };

  let mut res_buf = [0u8; 128];
  let mut read: u64 = 0;

  'read: loop {
    match stream.read(&mut res_buf[(read as usize)..]).await {
      Ok(0) => return Err(()),
      Ok(new_n) => {
        read += new_n as u64;
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut res = httparse::Response::new(&mut headers);
        match res.parse(&res_buf[0..read as usize]) {
          Ok(status) => match status {
            httparse::Status::Complete(head_n) => {
              let is_keepalive = 'k: {
                if !keepalive || res.version == Some(0) {
                  // if disabled keepalive in arguments or server http version is http/1.0 we are not using keepalive
                  break 'k false;
                } else {
                  for h in &headers {
                    if h.name.eq_ignore_ascii_case("connection") { 
                      match std::str::from_utf8(h.value) {
                        Ok(str) => {
                          for s in str.split(',') {
                            if s.trim().eq_ignore_ascii_case("close") {
                              // if the connection header contains "close" we are not using keepalive
                              break 'k false;
                            }
                          }

                          // if no "close" in the connection header we are using keepalive
                          break 'k true;
                        }

                        Err(_) => {
                          // if we cannot parse the connection header, we default to keepalive
                          break 'k true
                        }
                      }
                    }
                  }

                  // if no connection header, in http/1.1 default is keepalive
                  break 'k true;
                }
              };

              let content_length = 'content_length: {
                for h in &headers {
                  if !h.name.eq_ignore_ascii_case("content-length") {
                    continue;
                  }

                  let str = match std::str::from_utf8(h.value) {
                    Ok(str) => str,
                    Err(_) => return Err(()),
                  };

                  let len = match str.parse::<u64>() {
                    Ok(v) => v,
                    Err(_) => return Err(()),
                  };

                  break 'content_length Some(len);
                }

                None
              };

              match content_length {
                // identity encoding with content length
                Some(len) => {
                  let to_read = (len + head_n as u64).saturating_sub(read);

                  if to_read == 0 {
                    return Ok(is_keepalive);
                  }
    
                  match read_and_dispose(stream, to_read).await {
                    Ok(()) => return Ok(is_keepalive),
                    Err(_) => return Err(())
                  }
                }

                None => {
                  let is_chunked = headers.iter().any(|h| {
                    if !h.name.eq_ignore_ascii_case("transfer-encoding") {
                      return false;
                    }

                    let value= match std::str::from_utf8(h.value) {
                      Ok(v) => v,
                      Err(_) => return false,
                    };
                    
                    value.eq_ignore_ascii_case("chunked")
                  });

                  // chunked encoding
                  if is_chunked {

                    match consume_chunked_body(stream, &res_buf[head_n..read as usize]).await {
                      Ok(()) => return Ok(is_keepalive),
                      Err(_) => return Err(())
                    }

                  // no chunked encoding nor content-length, consume the response until the end and 
                  // dispose the connection, as in curl
                  } else {
                    match read_to_end(stream).await {
                      Ok(()) => return Ok(false),
                      Err(_) => return Err(())
                    }
                  }
                }
              }
            }

            httparse::Status::Partial => {
              continue 'read;
            }
          },

          Err(_) => return Err(()),
        }
      }

      Err(_) => return Err(()),
    }
  }
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