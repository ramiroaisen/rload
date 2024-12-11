use bytes::Buf;
use h2::client::SendRequest;

use crate::error::ErrorKind;

#[inline(always)]
pub async fn send_request<B: Buf>(
  mut h2: h2::client::SendRequest<B>,
  req: &'static http::Request<()>,
  #[cfg(feature = "timeout")] timeout: Option<std::time::Duration>,
) -> Result<SendRequest<B>, ErrorKind> {
  
  let inner = async move {
    h2 = match h2.ready().await {
      Ok(h2) => h2,
      Err(_) => return Err(ErrorKind::H2Ready),
    };

    let res = match h2.send_request(req.clone(), true) {
      Ok((res, _send_stream)) => res,
      Err(_) => return Err(ErrorKind::H2Send),
    };

    let res = match res.await {
      Ok(res) => res,
      Err(_) => return Err(ErrorKind::H2Recv),
    };

    let (_, mut body) = res.into_parts();

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
