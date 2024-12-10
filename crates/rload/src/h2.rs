use bytes::Buf;
use h2::client::SendRequest;

#[inline(always)]
pub async fn send_request<B: Buf>(
  mut h2: h2::client::SendRequest<B>,
  req: &'static http::Request<()>,
) -> Result<SendRequest<B>, ()> {
  
  h2 = match h2.ready().await {
    Ok(h2) => h2,
    Err(_) => return Err(()),
  };

  let res = match h2.send_request(req.clone(), true) {
    Ok((res, _send_stream)) => res,
    Err(_) => return Err(()),
  };

  let res = match res.await {
    Ok(res) => res,
    Err(_) => return Err(()),
  };

  let (_, mut body) = res.into_parts();

  while let Some(chunk) = body.data().await {
    match chunk {
      Ok(chunk) => {
        let _ = body.flow_control().release_capacity(chunk.len());
      }

      Err(_) => return Err(()),
    }
  }

  Ok(h2)
}
