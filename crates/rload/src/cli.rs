use clap::Parser;
use human_bytes::human_bytes;
use near_safe_cell::NearSafeCell;
use std::{net::SocketAddr, time::Duration};
use tokio::time::Instant;
use url::Url;

use crate::{
  args::{Args, Request, RunConfig},
  io::CounterStream,
};

#[derive(Debug, Clone)]
pub struct Report {
  pub url: Url,
  pub address: SocketAddr,
  pub http_version: HttpVersion,
  pub keepalive: bool,

  pub threads: usize,
  pub concurrency: usize,
  pub duration: Duration,
  pub elapsed: Duration,

  pub ok: u64,
  pub err: u64,
  pub read: u64,
  pub write: u64,
}

impl std::fmt::Display for Report {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let secs = self.elapsed.as_secs_f64();
    writeln!(f, "· url:          {}", self.url)?;
    writeln!(f, "· address:      {}", self.address)?;
    writeln!(f, "· http-version: {}", self.http_version)?;
    writeln!(
      f,
      "· keepalive:    {}",
      if self.keepalive {
        "enabled"
      } else {
        "disabled"
      }
    )?;

    writeln!(f, "· threads:      {}", self.threads)?;
    writeln!(f, "· concurrency:  {}", self.concurrency)?;
    writeln!(f, "· duration:     {}ms", self.duration.as_millis())?;

    writeln!(f, "· elapsed:      {}ms", self.elapsed.as_millis())?;
    writeln!(f, "· ok:           {}", self.ok)?;
    writeln!(f, "· errors:       {}", self.err)?;
    writeln!(
      f,
      "· read:         {} - {}/s",
      human_bytes(self.read as f64),
      human_bytes(self.read as f64 / secs)
    )?;
    writeln!(
      f,
      "· write:        {} - {}/s",
      human_bytes(self.write as f64),
      human_bytes(self.write as f64 / secs)
    )?;

    writeln!(
      f,
      "· req/s:        {}",
      (self.ok as f64 / secs).round() as u64
    )?;

    Ok(())
  }
}

#[derive(Debug, Clone, Copy)]
pub enum HttpVersion {
  #[cfg(feature = "h1")]
  Http1,
  #[cfg(feature = "h2")]
  Http2,
}

impl std::fmt::Display for HttpVersion {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      #[cfg(feature = "h1")]
      HttpVersion::Http1 => write!(f, "http/1"),
      #[cfg(feature = "h2")]
      HttpVersion::Http2 => write!(f, "h2"),
    }
  }
}

pub fn run() -> Result<Report, anyhow::Error> {
  let args = Args::parse();
  run_with_args(args)
}

pub fn run_with_args(args: Args) -> Result<Report, anyhow::Error> {
  let config = RunConfig::from_args(args)?;
  run_with_config(config)
}

pub fn run_with_config(config: RunConfig<'static>) -> Result<Report, anyhow::Error> {
  let start = Instant::now();
  let until = start + config.duration;

  let mut handles = Vec::with_capacity(config.threads);

  for _ in 0..config.threads {
    let handle = std::thread::spawn(move || thread(config, until));
    handles.push(handle);
  }

  let mut ok = 0;
  let mut err = 0;
  let mut read = 0;
  let mut write = 0;

  for handle in handles {
    let (thread_ok, thread_err, thread_read, thread_write) = handle.join().unwrap();
    ok += thread_ok;
    err += thread_err;
    read += thread_read;
    write += thread_write;
  }

  let elapsed = start.elapsed();

  let http_version = match config.request {
    #[cfg(feature = "h1")]
    Request::H1 { .. } => HttpVersion::Http1,
    #[cfg(feature = "h2")]
    Request::H2 { .. } => HttpVersion::Http2,
  };

  let report = Report {
    url: config.url.clone(),
    address: config.addr,
    http_version,
    keepalive: !config.no_keepalive,

    ok,
    err,
    read,
    write,

    threads: config.threads,
    concurrency: config.concurrency,
    duration: config.duration,
    elapsed,
  };

  Ok(report)
}

#[tokio::main(flavor = "current_thread")]
async fn thread(config: RunConfig<'static>, until: Instant) -> (u64, u64, u64, u64) {
  let read: &'static _ = Box::leak(Box::new(NearSafeCell::new(0u64)));
  let write: &'static _ = Box::leak(Box::new(NearSafeCell::new(0u64)));

  let conns = (config.concurrency as f64 / config.threads as f64).ceil() as usize;
  let mut handles = Vec::with_capacity(conns);
  for _ in 0..conns {
    let task = async move {
      let mut task_ok: u64 = 0;
      let mut task_err: u64 = 0;

      let task = async {
        'conn: loop {
          macro_rules! send_h1_requests {
            ($stream:ident, $buf:ident) => {{
              'req: loop {

                match crate::h1::send_request(&mut $stream, $buf, !config.no_keepalive).await {
                  Ok(is_keepalive) => {
                    task_ok += 1;
                    if !is_keepalive {
                      continue 'conn;
                    } else {
                      continue 'req;
                    }
                  }

                  Err(()) => {
                    task_err += 1;
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
                  task_err += 1;
                  continue 'conn;
                }
              };

              tokio::spawn(conn);

              'req: loop {

                match crate::h2::send_request(h2, $req).await {
                  Ok(sender) => {
                    h2 = sender;
                    task_ok += 1;
                    if config.no_keepalive {
                      continue 'conn;
                    } else {
                      continue 'req;
                    }
                  }

                  Err(()) => {
                    task_err += 1;
                    continue 'conn;
                  }
                }
              }
            }};
          }

          let stream = match tokio::net::TcpStream::connect(config.addr).await {
            Ok(stream) => stream,
            Err(_) => {
              task_err += 1;
              continue 'conn;
            }
          };

          let mut stream = CounterStream::new(stream, unsafe { read.get_mut_unsafe() }, unsafe { write.get_mut_unsafe()  });

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
            Some(never) => match never {} 

            #[cfg(feature = "tls")]
            Some(tls) => {
              let mut stream = match tls.connector.connect(tls.server_name.clone(), stream).await {
                Ok(stream) => stream,
                Err(_) => {
                  task_err += 1;
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

      let until = tokio::time::sleep_until(until);
      tokio::pin!(until);

      tokio::select! {
        _ = task => {}
        _ = &mut until => {}
      }

      (task_ok, task_err)
    };

    let handle = tokio::spawn(task);

    handles.push(handle);
  }

  let mut ok = 0;
  let mut err = 0;

  for handle in handles {
    let (task_ok, task_err) = handle.await.unwrap();
    ok += task_ok;
    err += task_err;
  }

  (ok, err, **read, **write)
}
