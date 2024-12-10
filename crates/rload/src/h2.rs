use bytes::Buf;
use h2::client::SendRequest;

#[inline(always)]
pub async fn send_request<B: Buf>(
  mut h2: h2::client::SendRequest<B>,
  req: &'static http::Request<()>,
) -> Result<(SendRequest<B>, u64, u64), (u64, u64)> {
  
  let mut read = 0;
  #[allow(unused_mut)]
  let mut write = 0;

  h2 = match h2.ready().await {
    Ok(h2) => h2,
    Err(_) => return Err((0, 0)),
  };

  let res = match h2.send_request(req.clone(), true) {
    Ok((res, _send_stream)) => res,
    Err(_) => return Err((read, write)),
  };

  let res = match res.await {
    Ok(res) => res,
    Err(_) => return Err((read, write)),
  };

  let (_, mut body) = res.into_parts();

  while let Some(chunk) = body.data().await {
    match chunk {
      Ok(chunk) => {
        read += chunk.len() as u64;
        let _ = body.flow_control().release_capacity(chunk.len());
      }

      Err(_) => return Err((read, write)),
    }
  }

  Ok((h2, read, write))
}
