use crate::{
  args::{Request, RunConfig}, error::{ErrorKind, Errors}, io::CounterStream, status::Statuses
};
use near_safe_cell::NearSafeCell;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct ThreadResult {
  pub ok: u64,
  pub read: u64,
  pub write: u64,
  #[cfg(feature = "latency")]
  pub hdr: hdrhistogram::Histogram<u64>,
  pub err: Errors,
  pub statuses: Statuses,
}

impl Default for ThreadResult {
  fn default() -> Self {
    Self {
      ok: 0,
      read: 0,
      write: 0,
      #[cfg(feature = "latency")]
      hdr: hdrhistogram::Histogram::<u64>::new(5).expect("error creating latency histogram"),
      err: Errors::new(),
      statuses: Statuses::new(),
    }
  }
}

#[tokio::main(flavor = "current_thread")]
pub async fn thread(config: RunConfig<'static>, cancel: CancellationToken) -> ThreadResult {
  macro_rules! leak {
    ($var:ident = $v:expr) => {
      let $var: &'static _ = Box::leak(Box::new(NearSafeCell::new($v)));
    };
  }

  leak!(result = ThreadResult::default());

  let conns = (config.concurrency as f64 / config.threads as f64).ceil() as usize;
  let mut handles = Vec::with_capacity(conns);
  for _ in 0..conns {
    let signal = cancel.clone().cancelled_owned();
    let task = async move {
      let task = async {
        'conn: loop {
          #[cfg(feature = "h1")]
          macro_rules! send_h1_requests {
            ($stream:ident, $buf:ident) => {{
              'req: loop {
                #[cfg(feature = "latency")]
                let start = {
                  if config.latency {
                    Some(std::time::Instant::now())
                  } else {
                    None
                  }
                };

                match crate::h1::send_request(
                  &mut $stream,
                  $buf,
                  !config.disable_keepalive,
                  unsafe { &mut result.get_mut_unsafe().statuses },
                  #[cfg(feature = "timeout")]
                  config.timeout,
                )
                .await
                {
                  Ok(is_keepalive) => {
                    unsafe {
                      result.get_mut_unsafe().ok += 1;
                    }
                    #[cfg(feature = "latency")]
                    {
                      if let Some(start) = start {
                        let elapsed = start.elapsed().as_nanos();
                        unsafe {
                          result.get_mut_unsafe().hdr.record(elapsed as u64).unwrap();
                        }
                      }
                    }

                    if !is_keepalive {
                      continue 'conn;
                    } else {
                      continue 'req;
                    }
                  }

                  Err(e) => {
                    unsafe {
                      result.get_mut_unsafe().err.record(e);
                    }
                    continue 'conn;
                  }
                }
              }
            }};
          }

          #[cfg(feature = "h2")]
          macro_rules! send_h2_requests {
            ($stream:ident, $req:ident) => {{
              let (mut h2, conn) = match h2::client::handshake($stream).await {
                Ok(pair) => pair,
                Err(_) => {
                  unsafe {
                    result.get_mut_unsafe().err.record(ErrorKind::H2Handshake);
                  }
                  continue 'conn;
                }
              };

              tokio::spawn(conn);

              'req: loop {
                #[cfg(feature = "latency")]
                let start = {
                  if config.latency {
                    Some(std::time::Instant::now())
                  } else {
                    None
                  }
                };

                match crate::h2::send_request(
                  h2,
                  || $req.clone(),
                  unsafe { &mut result.get_mut_unsafe().statuses },
                  #[cfg(feature = "timeout")]
                  config.timeout,
                )
                .await
                {
                  Ok(sender) => {
                    h2 = sender;
                    unsafe {
                      result.get_mut_unsafe().ok += 1;
                    }

                    #[cfg(feature = "latency")]
                    {
                      if let Some(start) = start {
                        let elapsed = start.elapsed().as_nanos();
                        unsafe {
                          result.get_mut_unsafe().hdr.record(elapsed as u64).unwrap();
                        }
                      }
                    }

                    if config.disable_keepalive {
                      continue 'conn;
                    } else {
                      continue 'req;
                    }
                  }

                  Err(e) => {
                    unsafe {
                      result.get_mut_unsafe().err.record(e);
                    }
                    continue 'conn;
                  }
                }
              }
            }};
          }

          let stream = match tokio::net::TcpStream::connect(config.addr).await {
            Ok(stream) => stream,
            Err(_) => {
              unsafe {
                result.get_mut_unsafe().err.record(ErrorKind::Connect);
              }
              continue 'conn;
            }
          };

          // Safety: this conters are local to this thread, so is not possible to race
          #[allow(unused_mut)]
          let mut stream = CounterStream::new(
            stream,
            unsafe { &mut result.get_mut_unsafe().read },
            unsafe { &mut result.get_mut_unsafe().write },
          );

          #[cfg(feature = "tls")]
          let tls = config.tls;

          #[cfg(not(feature = "tls"))]
          let tls = Option::<std::convert::Infallible>::None;

          match tls {
            None => match config.request {
              #[cfg(feature = "h1")]
              Request::H1 { buf } => {
                send_h1_requests!(stream, buf);
              }
              #[cfg(feature = "h2")]
              Request::H2 { req } => {
                send_h2_requests!(stream, req);
              }
            },

            #[cfg(not(feature = "tls"))]
            Some(never) => match never {},

            #[cfg(feature = "tls")]
            Some(tls) => {
              #[allow(unused_mut)]
              let mut stream = match tls.connector.connect(tls.server_name.clone(), stream).await {
                Ok(stream) => stream,
                Err(_) => {
                  unsafe {
                    result.get_mut_unsafe().err.record(ErrorKind::TlsHandshake);
                  }
                  continue 'conn;
                }
              };

              match config.request {
                #[cfg(feature = "h1")]
                Request::H1 { buf } => send_h1_requests!(stream, buf),
                #[cfg(feature = "h2")]
                Request::H2 { req } => send_h2_requests!(stream, req),
              }
            }
          }
        }
      };

      tokio::select! {
        _ = task => {}
        _ = signal => {}
      }
    };

    let handle = tokio::spawn(task);

    handles.push(handle);
  }

  for handle in handles {
    handle.await.unwrap();
  }

  macro_rules! unleak {
    ($var:ident) => {
      let $var = unsafe { *Box::from_raw($var.get_mut_ptr()) };
    };
  }

  unleak!(result);

  result
}
