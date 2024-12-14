use bytes::Bytes; 
use h2::client::SendRequest;

#[cfg(feature = "error-detail")]
use crate::error::ErrorKind;

#[cfg(feature = "status-detail")]
use crate::status::Statuses;

#[cfg(feature = "error-detail")]
type SendError = ErrorKind;

#[cfg(not(feature = "error-detail"))]
type SendError = ();

#[inline(always)]
pub async fn send_request(
  mut h2: h2::client::SendRequest<Bytes>,
  // we use a closure to avoid cloning the request in advance
  req: impl Fn() -> (http::Request<()>, Option<Bytes>),
  
  #[cfg(feature = "status-detail")]
  statuses: &mut Statuses,
  
  #[cfg(not(feature = "status-detail"))]
  not_ok_status: &mut u64,
  
  #[cfg(feature = "timeout")]
  timeout: Option<std::time::Duration>,
) -> Result<SendRequest<Bytes>, SendError> {
  
  macro_rules! err {
    ($err:ident) => {{
      #[cfg(feature = "error-detail")]
      {
        Err(SendError::$err)
      }
    
      #[cfg(not(feature = "error-detail"))]
      {
        Err(())
      }
    }}
  }

  let inner = async move {
    h2 = match h2.ready().await {
      Ok(h2) => h2,
      Err(_) => return err!(H2Ready),
    };

    let (req, body) = req();

    let has_body = body.is_some();
    let (res, mut send_stream) = match h2.send_request(req, !has_body) {
      Ok(pair) => pair,
      Err(_) => return err!(H2Send),
    };

    match body {
      None => {
        drop(send_stream);
      }
      Some(bytes) => {
        tokio::spawn(async move {
          send_stream.reserve_capacity(bytes.len());
          send_stream.send_data(bytes, true)
        });
      }
    }      

    let res = match res.await {
      Ok(res) => res,
      Err(_) => return err!(H2Recv),
    };

    #[cfg(feature = "status-detail")]
    unsafe {
      // Safety: the maximum u16 value for an http::StatusCode code is 999
      statuses.record_unchecked(res.status().as_u16())
    };

    #[cfg(not(feature = "status-detail"))]
    {
      let status = res.status().as_u16();
      if !matches!(status, 200..=399) {
        *not_ok_status += 1;
      }
    }

    let mut body = res.into_body();

    while let Some(chunk) = body.data().await {
      match chunk {
        Ok(chunk) => {
          let _ = body.flow_control().release_capacity(chunk.len());
        }

        Err(_) => return err!(H2Body),
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
        // note that pingora timeouts will ceil to the next 10ms
        match pingora_timeout::timeout(timeout, inner).await {
          Ok(res) => res,
          Err(_) => err!(Timeout),
        }
      } 

      None => inner.await
    }
  }
}
