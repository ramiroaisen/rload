use std::task::{Context, Poll};
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[pin_project]
#[derive(Debug)]
pub struct CounterStream<'a, S> {
  read: &'a mut u64,
  write: &'a mut u64,
  #[pin]
  inner: S,
}

impl<'a, S> CounterStream<'a, S> {
  pub fn new(inner: S, read: &'a mut u64, write: &'a mut u64) -> Self {
    Self {
      read,
      write,
      inner,
    }
  }
}




impl<S: AsyncRead> AsyncRead for CounterStream<'_, S> {
  #[inline(always)]
  fn poll_read(
    self: std::pin::Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &mut ReadBuf<'_>,
  ) -> Poll<std::io::Result<()>> {
    let mut me = self.project();
    match me.inner.as_mut().poll_read(cx, buf) {
      Poll::Pending => Poll::Pending,
      Poll::Ready(Err(e)) => Poll::Ready(Err(e)), 
      Poll::Ready(Ok(())) => {
        // unsafe { *me.read.get_mut_unsafe() += buf.filled().len() as u64 };
        **me.read += buf.filled().len()as u64;
        Poll::Ready(Ok(()))
      }
    }
  }
}

impl<S: AsyncWrite> AsyncWrite for CounterStream<'_, S> {
  #[inline(always)]
  fn poll_write(
    self: std::pin::Pin<&mut Self>,
    cx: &mut Context<'_>,
    buf: &[u8],
  ) -> Poll<std::io::Result<usize>> {
    let mut me = self.project();
    match me.inner.as_mut().poll_write(cx, buf) {
      Poll::Pending => Poll::Pending,
      Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
      Poll::Ready(Ok(n)) => {
        // unsafe { *me.write.get_mut_unsafe() += n as u64 };
        **me.write += n as u64;
        Poll::Ready(Ok(n))
      }
    }
  }

  #[inline(always)]
  fn poll_write_vectored(
    self: std::pin::Pin<&mut Self>,
    cx: &mut Context<'_>,
    bufs: &[std::io::IoSlice<'_>],
  ) -> Poll<std::io::Result<usize>> {
    let mut me = self.project();
    match me.inner.as_mut().poll_write_vectored(cx, bufs) {
      Poll::Pending => Poll::Pending,
      Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
      Poll::Ready(Ok(n)) => {
        // unsafe { *me.write.get_mut_unsafe() += n as u64 };
        **me.write += n as u64;
        Poll::Ready(Ok(n))
      }
    }
  }

  #[inline(always)]
  fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
    let mut me = self.project();
    me.inner.as_mut().poll_flush(cx)
  }

  #[inline(always)]
  fn poll_shutdown(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
    let mut me = self.project();
    me.inner.as_mut().poll_shutdown(cx)
  }

  #[inline(always)]
  fn is_write_vectored(&self) -> bool {
    self.inner.is_write_vectored()
  }
}


#[cfg(feature = "monoio")]
impl<S: monoio::io::AsyncReadRent> monoio::io::AsyncReadRent for CounterStream<'_, S> {
  async fn read<T: monoio::buf::IoBufMut>(&mut self, buf: T) -> monoio::BufResult<usize, T> {
    match self.inner.read(buf).await {
      (Ok(n), buf) => {
        *self.read += n as u64;
        (Ok(n), buf)
      }
      other => other,
    }
  }

  async fn readv<T: monoio::buf::IoVecBufMut>(&mut self, buf: T) -> monoio::BufResult<usize, T> {
    match self.inner.readv(buf).await {
      (Ok(n), buf) => {
        *self.read += n as u64;
        (Ok(n), buf)
      }
      other => other,
    }
  }
}


#[cfg(feature = "monoio")]
impl<S: monoio::io::AsyncWriteRent> monoio::io::AsyncWriteRent for CounterStream<'_, S> {
  async fn write<T: monoio::buf::IoBuf>(&mut self, buf: T) -> monoio::BufResult<usize, T> {
    match self.inner.write(buf).await {
      (Ok(n), buf) => {
        *self.write += n as u64;
        (Ok(n), buf)
      }
      other => other,
    }
  }

  async fn writev<T: monoio::buf::IoVecBuf>(&mut self, buf: T) -> monoio::BufResult<usize, T> {
    match self.inner.writev(buf).await {
      (Ok(n), buf) => {
        *self.write += n as u64;
        (Ok(n), buf)
      }
      other => other,
    }
  } 

  async fn flush(&mut self) -> std::io::Result<()> {
    self.inner.flush().await
  }

  async fn shutdown(&mut self) -> std::io::Result<()> {
    self.inner.shutdown().await
  }
}