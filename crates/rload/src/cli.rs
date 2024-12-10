#[cfg(feature = "latency")]
use anyhow::Context;
use clap::Parser;
use human_bytes::human_bytes;
#[cfg(feature = "latency")]
use humantime::FormattedDuration;
use near_safe_cell::NearSafeCell;
use tokio_util::sync::CancellationToken;
use std::{net::SocketAddr, thread, time::Duration};
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

  #[cfg(feature = "latency")]
  pub hdr: Option<hdrhistogram::Histogram<u64>>,
}

impl std::fmt::Display for Report {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let secs = self.elapsed.as_secs_f64();
    
    writeln!(f)?;
    writeln!(f, "==========| Config |=========")?;
    writeln!(f, "url:          {}", self.url)?;
    writeln!(f, "address:      {}", self.address)?;
    writeln!(f, "http-version: {}", self.http_version)?;
    writeln!(f, "keepalive:    {}",
      if self.keepalive {
        "enabled"
      } else {
        "disabled"
      }
    )?;

    writeln!(f, "threads:      {}", self.threads)?;
    writeln!(f, "concurrency:  {}", self.concurrency)?;
    writeln!(f, "duration:     {}", humantime::format_duration(self.duration))?; 

    #[cfg(feature = "latency")]
    {
      fn t(nanos: u64) -> FormattedDuration {
        humantime::format_duration(Duration::from_nanos(nanos))
      }

      fn tf(nanos: f64) -> FormattedDuration {
        humantime::format_duration(Duration::from_nanos(nanos.round() as u64))
      }


      if let Some(hdr) = &self.hdr {        
        
        let avg = {
          let sum = hdr.iter_recorded().fold(0u64, |t, v| t + v.count_at_value() * v.value_iterated_to());
          sum as f64 / hdr.len() as f64          
        };

        writeln!(f)?;
        writeln!(f, "=========| Latency |=========")?;
        writeln!(f, "min:     {}", t(hdr.min()))?;
        writeln!(f, "max:     {}", t(hdr.max()))?;
        writeln!(f, "avg:     {}", tf(avg))?; 
        writeln!(f, "mean:    {}", tf(hdr.mean()))?;
        writeln!(f, "stdev:   {}", tf(hdr.stdev()))?;
        writeln!(f, "-----------------------------")?;


        writeln!(f, "50.000%   {}", t(hdr.value_at_percentile(50.0)))?;
        writeln!(f, "75.000%   {}", t(hdr.value_at_percentile(75.0)))?;
        writeln!(f, "90.000%   {}", t(hdr.value_at_percentile(95.0)))?;
        writeln!(f, "99.000%   {}", t(hdr.value_at_percentile(99.0)))?;
        writeln!(f, "99.900%   {}", t(hdr.value_at_percentile(99.9)))?;
        writeln!(f, "99.990%   {}", t(hdr.value_at_percentile(99.99)))?;
        writeln!(f, "99.999%   {}", t(hdr.value_at_percentile(99.999)))?;
      }
    }

    writeln!(f)?;
    writeln!(f, "==========| Result |=========")?;
    writeln!(f, "elapsed:      {}", humantime::format_duration(self.elapsed))?;
    writeln!(f, "requests:     {}", self.ok)?;
    writeln!(f, "errors:       {}", self.err)?;
    writeln!(
      f,
      "read:         {} - {}/s",
      human_bytes(self.read as f64),
      human_bytes(self.read as f64 / secs)
    )?;
    writeln!(
      f,
      "write:        {} - {}/s",
      human_bytes(self.write as f64),
      human_bytes(self.write as f64 / secs)
    )?;

    writeln!(
      f,
      "requests/sec: {}",
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
  let cancel = CancellationToken::new();

  for _ in 0..config.threads {
    let cancel = cancel.clone();
    let handle = std::thread::spawn(move || thread(config, cancel));
    handles.push(handle);
  }

  thread::spawn(move || {
    watch_cancel(cancel, until);
  });

  let mut ok = 0;
  let mut err = 0;
  let mut read = 0;
  let mut write = 0;
  
  #[cfg(feature = "latency")]
  let mut hdr = hdrhistogram::Histogram::<u64>::new(5).expect("error creating latency histogram");
 
  let results = handles.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>();
  let elapsed = start.elapsed();

  for t in results {
    ok += t.ok;
    err += t.err;
    read += t.read;
    write += t.write;
    #[cfg(feature = "latency")]
    {
      if config.latency {
        hdr.add(t.hdr).context("error adding latency histogram to the final result")?;
      }
    }
  }

  let http_version = match config.request {
    #[cfg(feature = "h1")]
    Request::H1 { .. } => HttpVersion::Http1,
    #[cfg(feature = "h2")]
    Request::H2 { .. } => HttpVersion::Http2,
  };

  #[cfg(feature = "latency")]
  let hdr = match config.latency {
    true => Some(hdr),
    false => None,
  };

  let report = Report {
    url: config.url.clone(),
    address: config.addr,
    http_version,
    keepalive: !config.disable_keepalive,

    ok,
    err,
    read,
    write,

    threads: config.threads,
    concurrency: config.concurrency,
    duration: config.duration,
    elapsed,

    #[cfg(feature = "latency")]
    hdr,
  };

  Ok(report)
}

#[tokio::main(flavor = "current_thread")]
async fn watch_cancel(cancel: CancellationToken, until: Instant) {
  let ctrl_c = tokio::signal::ctrl_c();
  let timer = tokio::time::sleep_until(until);
  tokio::select! {
    _ = ctrl_c => {}
    _ = timer => {}
  };
  cancel.cancel();
}

#[derive(Debug, Clone)]
struct ThreadResult {
  ok: u64,
  err: u64,
  read: u64,
  write: u64,
  #[cfg(feature = "latency")]
  hdr: &'static hdrhistogram::Histogram<u64>,
}

#[tokio::main(flavor = "current_thread")]
async fn thread(config: RunConfig<'static>, cancel: CancellationToken) -> ThreadResult {
  let read: &'static _ = Box::leak(Box::new(NearSafeCell::new(0u64)));
  let write: &'static _ = Box::leak(Box::new(NearSafeCell::new(0u64)));
  #[cfg(feature = "latency")]
  let hdr: &'static _ = Box::leak(Box::new(NearSafeCell::new(hdrhistogram::Histogram::<u64>::new(5).expect("error creating latency histogram"))));
  
  let conns = (config.concurrency as f64 / config.threads as f64).ceil() as usize;
  let mut handles = Vec::with_capacity(conns);
  for _ in 0..conns {
    let signal = cancel.clone().cancelled_owned();
    let task = async move {
      let mut task_ok: u64 = 0;
      let mut task_err: u64 = 0;

      let task = async {
        'conn: loop {

          #[cfg(feature = "h1")]
          macro_rules! send_h1_requests {
            ($stream:ident, $buf:ident) => {{
              'req: loop {

                #[cfg(feature = "latency")]
                let start = {
                  if config.latency {
                    Some(Instant::now())
                  } else {
                    None
                  }
                };

                match crate::h1::send_request(&mut $stream, $buf, !config.disable_keepalive).await {
                  Ok(is_keepalive) => {
                    task_ok += 1;
                    #[cfg(feature = "latency")]
                    {
                      if let Some(start) = start {
                        let elapsed = start.elapsed().as_nanos();
                        unsafe { hdr.get_mut_unsafe().record(elapsed as u64).unwrap() };
                      }
                    }

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
                    if config.disable_keepalive {
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

          #[allow(unused_mut)]
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
              #[allow(unused_mut)]
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

      // let until = tokio::time::sleep_until(until);
      // tokio::pin!(until);

      tokio::select! {
        _ = task => {}
        _ = signal => {}
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

  ThreadResult {
    ok,
    err,
    read: **read,
    write: **write,
    #[cfg(feature = "latency")]
    hdr,
  }
}
