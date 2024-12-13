#[cfg(feature = "latency")]
use anyhow::Context;
use clap::Parser;
use std::{thread, time::Duration};
use tokio::{sync::watch, time::Instant};

use crate::{
  args::{Args, Request, RunConfig}, error::Errors, http, report::Report, status::Statuses
};

pub fn run() -> Result<Report, anyhow::Error> {
  let args = Args::parse();
  run_with_args(args)
}

pub fn run_with_args(args: Args) -> Result<Report, anyhow::Error> {
  let config = RunConfig::from_args(args)?;
  run_with_config(config)
}

pub fn run_with_config(config: RunConfig<'static>) -> Result<Report, anyhow::Error> {
  eprintln!(
    "Running {} test @ {}",
    crate::fmt::format_duration(config.duration),
    config.url
  );
  eprintln!(
    "  {} threads and {} connections",
    config.threads, config.concurrency
  );

  let mut handles = Vec::with_capacity(config.threads);

  // with this signaling to start processing we gain a little in precision of the time measuring
  let (start_send, start_recv) = watch::channel(());

  let (stop_send, stop_recv) = watch::channel(());


  for _ in 0..config.threads {
    let start = start_recv.clone();
    let stop = stop_recv.clone();
    let handle = std::thread::spawn(move || crate::run::thread(config, start, stop));
    handles.push(handle);
  }

  drop(start_recv);

  let duration = config.duration;
  let start = thread::spawn(move || {
    // give the threads time to startup
    thread::sleep(Duration::from_millis(25));
    let start = Instant::now();
    let until = start + duration;
    start_send.send(()).unwrap();
    watch_stop(stop_send, until);
    start
  }).join().unwrap();

  let mut ok = 0;
  let mut err = Errors::new();
  let mut read = 0;
  let mut write = 0;
  let mut statuses = Statuses::new();

  #[cfg(feature = "latency")]
  let mut hdr = hdrhistogram::Histogram::<u64>::new(5).expect("error creating latency histogram");

  let results = handles
    .into_iter()
    .map(|h| h.join().unwrap())
    .collect::<Vec<_>>();
  let elapsed = start.elapsed();

  for t in results {
    ok += t.ok;
    err.join(t.err);
    read += t.read;
    write += t.write;
    statuses.join(t.statuses);
    #[cfg(feature = "latency")]
    {
      if config.latency {
        hdr
          .add(t.hdr)
          .context("error adding latency histogram to the final result")?;
      }
    }
  }

  let http_version = match config.request {
    #[cfg(feature = "h1")]
    Request::H1 { .. } => http::Version::Http1,
    #[cfg(feature = "h2")]
    Request::H2 { .. } => http::Version::Http2,
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
    method: config.method.into(),
    body_len: config.body_len,
    ok,
    err,
    read,
    write,
    statuses: statuses.iter().collect(),

    threads: config.threads,
    concurrency: config.concurrency,
    duration: config.duration,

    #[cfg(feature = "timeout")]
    timeout: config.timeout,

    elapsed,

    #[cfg(feature = "latency")]
    hdr,
  };

  Ok(report)
}

#[tokio::main(flavor = "current_thread")]
async fn watch_stop(stop: watch::Sender<()>, until: Instant) {
  let ctrl_c = tokio::signal::ctrl_c();
  let timer = tokio::time::sleep_until(until);
  tokio::select! {
    _ = ctrl_c => {}
    _ = timer => {}
  };
  let _ = stop.send(());
}
