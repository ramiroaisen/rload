use bytes::Buf;
use h2::client::SendRequest;

use crate::{error::ErrorKind, status::Statuses};

#[inline(always)]
pub async fn send_request<B: Buf>(
  mut h2: h2::client::SendRequest<B>,
  // we use a closure to avoid cloning the request in advance
  req: impl Fn() -> http::Request<()>,
  statuses: &mut Statuses,
  #[cfg(feature = "timeout")]
  timeout: Option<std::time::Duration>,
) -> Result<SendRequest<B>, ErrorKind> {
  
  let inner = async move {
    h2 = match h2.ready().await {
      Ok(h2) => h2,
      Err(_) => return Err(ErrorKind::H2Ready),
    };

    let res = match h2.send_request(req(), true) {
      Ok((res, _send_stream)) => res,
      Err(_) => return Err(ErrorKind::H2Send),
    };

    let res = match res.await {
      Ok(res) => res,
      Err(_) => return Err(ErrorKind::H2Recv),
    };

    unsafe {
      // Safety: the maximum u16 value for an http::StatusCode code is 999
      statuses.record_unchecked(res.status().as_u16())
    };

    let mut body = res.into_body();

    while let Some(chunk) = body.data().await {
      match chunk {
        Ok(chunk) => {
          let _ = body.flow_control().release_capacity(chunk.len());
        }

        Err(_) => return Err(ErrorKind::H2Body),
      }
    }

    Ok(h2)
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
