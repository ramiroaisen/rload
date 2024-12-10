use std::{mem::MaybeUninit, pin::Pin, task::{Context, Poll}};

use httparse::parse_chunk_size;
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

#[inline(always)]
pub async fn send_request<S: AsyncRead + AsyncWrite + Unpin>(
  stream: &mut S,
  req_buf: &[u8],
) -> Result<(u64, u64), (u64, u64)> {
  match stream.write_all(req_buf).await {
    Ok(()) => {}
    Err(_) => return Err((0, 0)),
  };

  let write = req_buf.len() as u64;

  let mut res_buf = [0u8; 1024 * 16];
  let mut read = 0;

  'read: loop {
    match stream.read(&mut res_buf[read..]).await {
      Ok(0) => return Err((read as u64, write)),
      Ok(new_n) => {
        read += new_n;
        let mut headers = [httparse::EMPTY_HEADER; 16];
        match httparse::Response::new(&mut headers).parse(&res_buf[0..read]) {
          Ok(status) => match status {
            httparse::Status::Complete(head_n) => {
              let content_length = 'content_length: {
                for h in &headers {
                  if !h.name.eq_ignore_ascii_case("content-length") {
                    continue;
                  }

                  let str = match std::str::from_utf8(h.value) {
                    Ok(str) => str,
                    Err(_) => return Err((read as u64, write)),
                  };

                  let len = match str.parse::<usize>() {
                    Ok(v) => v,
                    Err(_) => return Err((read as u64, write)),
                  };

                  break 'content_length len;
                }

                0
              };

              // let is_chunked = headers.iter().any(|h| {
              //   if !h.name.eq_ignore_ascii_case("transfer-encoding") {
              //     return false;
              //   }

              //   let value= match str::from_utf8(h.value) {
              //     Ok(v) => v,
              //     Err(_) => return false,
              //   };
                
              //   value.eq_ignore_ascii_case("chunked")
              // });

              // if is_chunked {
              //   // TODO: dispose chunked body

              // } else {
                let to_read = (content_length + head_n).saturating_sub(read);

                if to_read == 0 {
                  return Ok((read as u64, write));
                }

                match read_and_dispose(stream).await {
                  Ok(n) => return Ok((read as u64 + n, write)),
                  Err((n, _)) => return Err((read as u64 + n, write))
                }
              // } 
            }

            httparse::Status::Partial => {
              continue 'read;
            }
          },

          Err(_) => return Err((read as u64, write)),
        }
      }

      Err(_) => return Err((read as u64, write)),
    }
  }
}

#[inline(always)]
fn read_and_dispose<R: AsyncRead>(r: R) -> ReadAndDispose<R> {
  ReadAndDispose {
    inner: r,
    n: 0,
  }
}

#[pin_project]
struct ReadAndDispose<R> {
  #[pin]
  inner: R,
  n: u64,
}

impl<R: AsyncRead> std::future::Future for ReadAndDispose<R> {
  type Output = Result<u64, (u64, std::io::Error)>;

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
        Poll::Ready(Err(e)) => return Poll::Ready(Err((*me.n, e))),
        Poll::Ready(Ok(())) => {
          if buf.filled().is_empty() {
            return Poll::Ready(Ok(*me.n));
          } else {
            *me.n += buf.filled().len() as u64;
            continue;
          }
        }
      }
    }
  }
}