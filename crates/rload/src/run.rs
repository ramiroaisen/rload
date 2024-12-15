use crate::{args::{Request, RunConfig}, io::CounterStream};

#[cfg(feature = "error-detail")]
use crate::error::{ErrorKind, Errors};

#[cfg(feature = "status-detail")]
use crate::status::Statuses;

use near_safe_cell::NearSafeCell;
use tokio::sync::watch;

#[derive(Debug, Clone)]
pub struct ThreadResult {
  pub ok: u64,
  pub read: u64,
  pub write: u64,
  #[cfg(feature = "latency")]
  pub hdr: hdrhistogram::Histogram<u64>,

  #[cfg(feature = "error-detail")]
  pub err: Errors,
  #[cfg(not(feature = "error-detail"))]
  pub err_count: u64,

  #[cfg(feature = "status-detail")]
  pub statuses: Statuses,
  #[cfg(not(feature = "status-detail"))]
  pub not_ok_status: u64,
}

impl Default for ThreadResult {
  fn default() -> Self {
    Self {
      ok: 0,
      read: 0,
      write: 0,
      #[cfg(feature = "latency")]
      hdr: hdrhistogram::Histogram::<u64>::new(5).expect("error creating latency histogram"),
      
      #[cfg(feature = "error-detail")]
      err: Errors::new(),
      #[cfg(not(feature = "error-detail"))]
      err_count: 0,


      #[cfg(feature = "status-detail")]
      statuses: Statuses::new(),
      #[cfg(not(feature = "status-detail"))]
      not_ok_status: 0,
    }
  }
}


#[cfg(feature = "monoio")]
#[monoio::main(driver = "legacy", timer = true)]
pub async fn thread(
  config: RunConfig<'static>,
  start: watch::Receiver<()>,
  stop: watch::Receiver<()>,
) -> ThreadResult {
  thread_inner(config, start, stop).await
}

#[cfg(not(feature = "monoio"))]
#[tokio::main(flavor = "current_thread")]
pub async fn thread(
  config: RunConfig<'static>,
  start: watch::Receiver<()>,
  stop: watch::Receiver<()>,
) -> ThreadResult {
  thread_inner(config, start, stop).await
}

pub async fn thread_inner(
  config: RunConfig<'static>,
  start: watch::Receiver<()>,
  stop: watch::Receiver<()>,
) -> ThreadResult {
  macro_rules! leak {
    ($var:ident = $v:expr) => {
      let $var: &'static _ = Box::leak(Box::new(NearSafeCell::new($v)));
    };
  }

  leak!(result = ThreadResult::default());

  let conns = (config.concurrency as f64 / config.threads as f64).ceil() as usize;
  let mut handles = Vec::with_capacity(conns);
  for _ in 0..conns {
    let mut stop = stop.clone();
    let mut start = start.clone();
    let task = async move {
      let task = async {
        
        start.changed().await.unwrap();

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

                  #[cfg(feature = "status-detail")]
                  unsafe { &mut result.get_mut_unsafe().statuses },
                 
                  #[cfg(not(feature = "status-detail"))]
                  unsafe { &mut result.get_mut_unsafe().not_ok_status },

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
                  #[allow(unused)]
                  Err(e) => {
                    #[cfg(feature = "error-detail")]
                    unsafe {
                      result.get_mut_unsafe().err.record(e);
                    }

                    #[cfg(not(feature = "error-detail"))]
                    unsafe {
                      result.get_mut_unsafe().err_count += 1;
                    }

                    continue 'conn;
                  }
                }
              }
            }};
          }

          #[cfg(feature = "h2")]
          macro_rules! send_h2_requests {
            ($stream:ident, $req:ident, $body:ident) => {{
              let (mut h2, conn) = match crate::rt::h2::client::handshake($stream).await {
                Ok(pair) => pair,
                Err(_) => {
                  #[cfg(feature = "error-detail")]
                  unsafe {
                    result.get_mut_unsafe().err.record(ErrorKind::H2Handshake);
                  }

                  #[cfg(not(feature = "error-detail"))]
                  unsafe {
                    result.get_mut_unsafe().err_count += 1;
                  }

                  continue 'conn;
                }
              };

              crate::rt::spawn(conn);

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
                  || ($req.clone(), $body.cloned()),

                  #[cfg(feature = "status-detail")]
                  unsafe { &mut result.get_mut_unsafe().statuses },

                  #[cfg(not(feature = "status-detail"))]
                  unsafe { &mut result.get_mut_unsafe().not_ok_status },

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

                  #[allow(unused)]
                  Err(e) => {
                    #[cfg(feature = "error-detail")]
                    unsafe {
                      result.get_mut_unsafe().err.record(e);
                    }

                    #[cfg(not(feature = "error-detail"))]
                    unsafe {
                      result.get_mut_unsafe().err_count += 1;
                    }

                    continue 'conn;
                  }
                }
              }
            }};
          }

          macro_rules! timeout {
            ($inner:expr, $err:ident) => {{
              #[cfg(not(feature = "timeout"))]
              match $inner.await {
                Ok(stream) => stream,
                Err(_) => {
                  #[cfg(feature = "error-detail")]
                  unsafe {
                    result.get_mut_unsafe().err.record(ErrorKind::$err);
                  }

                  #[cfg(not(feature = "error-detail"))]
                  unsafe {
                    result.get_mut_unsafe().err_count += 1;
                  }

                  continue 'conn;
                }
              }

              #[cfg(feature = "timeout")]
              match config.timeout {
                None => match $inner.await {
                  Ok(stream) => stream,
                  Err(_) => {
                    #[cfg(feature = "error-detail")]
                    unsafe {
                      result.get_mut_unsafe().err.record(ErrorKind::$err);
                    }

                    #[cfg(not(feature = "error-detail"))]
                    unsafe {
                      result.get_mut_unsafe().err_count += 1;
                    }

                    continue 'conn;
                  }
                }

                Some(timeout) => {
                  match pingora_timeout::timeout(timeout, $inner).await {
                    Ok(Ok(stream)) => stream,
                    Ok(Err(_)) => {
                      #[cfg(feature = "error-detail")]
                      unsafe {
                        result.get_mut_unsafe().err.record(ErrorKind::$err);
                      }

                      #[cfg(not(feature = "error-detail"))]
                      unsafe {
                        result.get_mut_unsafe().err_count += 1;
                      }

                      continue 'conn;
                    }
                    Err(_) => {
                      #[cfg(feature = "error-detail")]
                      unsafe {
                        result.get_mut_unsafe().err.record(ErrorKind::Timeout);
                      }

                      #[cfg(not(feature = "error-detail"))]
                      unsafe {
                        result.get_mut_unsafe().err_count += 1;
                      }

                      continue 'conn;
                    }
                  }
                }
              }
            }};
          }

          let stream = timeout!(crate::rt::TcpStream::connect(config.addr), Connect);

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
              Request::H2 { req, body } => {
                send_h2_requests!(stream, req, body);
              }
            },

            #[cfg(not(feature = "tls"))]
            Some(never) => match never {},

            #[cfg(feature = "tls")]
            Some(tls) => {

              #[allow(unused_mut)]
              let mut stream = timeout!(tls.connector.connect(tls.server_name.clone(), stream), TlsHandshake);

              match config.request {
                #[cfg(feature = "h1")]
                Request::H1 { buf } => send_h1_requests!(stream, buf),
                #[cfg(feature = "h2")]
                Request::H2 { req, body } => send_h2_requests!(stream, req, body),
              }
            }
          }
        }
      };

      

      tokio::select! {
        biased;
        _ = task => {}
        _ = stop.changed() => {}
      }
    };

    let handle = crate::rt::spawn(task);

    handles.push(handle);
  }

  for handle in handles {
    #[cfg(feature = "monoio")]
    handle.await;

    #[cfg(not(feature = "monoio"))]
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
