use human_bytes::human_bytes;
use std::{net::SocketAddr, time::Duration};
use url::Url;

use crate::fmt::format_duration;

#[cfg(feature = "error-detail")]
use crate::error::Errors;

#[derive(Debug, Clone)]
pub struct Report {
  pub url: Url,
  pub address: SocketAddr,
  pub http_version: crate::http::Version,
  pub keepalive: bool,
  pub method: String,
  pub body_len: usize,

  pub threads: usize,
  pub concurrency: usize,
  pub duration: Duration,
  pub elapsed: Duration,

  #[cfg(feature = "timeout")]
  pub timeout: Option<Duration>,

  pub ok: u64,
  pub read: u64,
  pub write: u64,

  #[cfg(feature = "error-detail")]
  pub err: Errors,

  #[cfg(not(feature = "error-detail"))]
  pub err_count: u64,

  #[cfg(feature = "status-detail")]
  pub statuses: Vec<(u16, u64)>,

  #[cfg(not(feature = "status-detail"))]
  pub not_ok_status: u64,

  #[cfg(feature = "latency")]
  pub hdr: Option<hdrhistogram::Histogram<u64>>,
}

impl std::fmt::Display for Report {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let secs = self.elapsed.as_secs_f64();

    if self.ok == 0 {
      write!(
        f,
        "no data was collected, all requests failed or didn't complete",
      )?;
    } else {
      write!(
        f,
        " {} requests in {}, {} read, {} write",
        self.ok,
        format_duration(self.elapsed),
        human_bytes(self.read as f64),
        human_bytes(self.write as f64),
      )?;
    }

    writeln!(f)?;
    writeln!(f, "==========| Config |=========")?;
    writeln!(f, "url:          {}", self.url)?;
    writeln!(f, "address:      {}", self.address)?;
    writeln!(f, "http-version: {}", self.http_version)?;
    writeln!(f, "method:       {}", self.method)?;
    if self.body_len != 0
      || !matches!(
        self.method.as_ref(),
        "GET" | "HEAD" | "DELETE" | "OPTIONS" | "TRACE"
      )
    {
      writeln!(f, "body:         {}", human_bytes(self.body_len as f64))?;
    }
    writeln!(
      f,
      "keepalive:    {}",
      if self.keepalive {
        "enabled"
      } else {
        "disabled"
      }
    )?;

    writeln!(f, "threads:      {}", self.threads)?;
    writeln!(f, "concurrency:  {}", self.concurrency)?;
    writeln!(
      f,
      "duration:     {}",
      crate::fmt::format_duration(self.duration)
    )?;
    #[cfg(feature = "timeout")]
    if let Some(timeout) = self.timeout {
      writeln!(f, "timeout:      {}", crate::fmt::format_duration(timeout))?;
    }

    writeln!(f, "runtime:      {}", crate::rt::NAME)?;

    #[cfg(feature = "latency")]
    {
      fn t(nanos: u64) -> crate::fmt::FormatDuration {
        crate::fmt::format_duration(Duration::from_nanos(nanos))
      }

      fn tf(nanos: f64) -> crate::fmt::FormatDuration {
        crate::fmt::format_duration(Duration::from_nanos(nanos.round() as u64))
      }

      if let Some(hdr) = &self.hdr {
        writeln!(f)?;
        writeln!(f, "=========| Latency |=========")?;
        writeln!(f, "min:     {}", t(hdr.min()))?;
        writeln!(f, "max:     {}", t(hdr.max()))?;
        writeln!(f, "mean:    {}", tf(hdr.mean()))?;
        writeln!(f, "stdev:   {}", tf(hdr.stdev()))?;
        writeln!(f, "-----------------------------")?;

        writeln!(f, "50%      {}", t(hdr.value_at_percentile(50.0)))?;
        writeln!(f, "75%      {}", t(hdr.value_at_percentile(75.0)))?;
        writeln!(f, "90%      {}", t(hdr.value_at_percentile(95.0)))?;
        writeln!(f, "99%      {}", t(hdr.value_at_percentile(99.0)))?;
        writeln!(f, "99.9%    {}", t(hdr.value_at_percentile(99.9)))?;
        writeln!(f, "99.99%   {}", t(hdr.value_at_percentile(99.99)))?;
        writeln!(f, "99.999%  {}", t(hdr.value_at_percentile(99.999)))?;
      }
    }

    writeln!(f)?;
    writeln!(f, "==========| Result |=========")?;
    writeln!(
      f,
      "elapsed:            {}",
      crate::fmt::format_duration(self.elapsed)
    )?;
    writeln!(f, "fulfilled:          {}", self.ok)?;

    #[cfg(feature = "error-detail")]
    {
      let total = self.err.total();
      if total == 0 {
        writeln!(f, "errors:             0")?;
      } else {
        writeln!(f, "- errors")?;
        fn err(
          f: &mut std::fmt::Formatter<'_>,
          name: impl std::fmt::Display,
          count: u64,
        ) -> std::fmt::Result {
          if count != 0 {
            writeln!(f, "  · {: <15}{}", format!("{}:", name), count)?;
          }

          Ok(())
        }

        err(f, "total", total)?;
        for (kind, count) in self.err.iter() {
          err(f, kind, count)?;
        }
      }
    }

    #[cfg(not(feature = "error-detail"))]
    {
      println!("errors:             {}", self.err_count);
    }

    #[cfg(feature = "status-detail")]
    {
      let has_non_200 = self.statuses.iter().any(|(status, _)| *status != 200);
      if has_non_200 {
        writeln!(f, "- Status codes")?;
        for (status, count) in self.statuses.iter() {
          writeln!(f, "  · {}: {}", status, count)?;
        }
      }
    }

    #[cfg(not(feature = "status-detail"))]
    {
      if self.not_ok_status != 0 {
        println!("not 2xx/3xx status: {}", self.not_ok_status);
      }
    }

    writeln!(
      f,
      "read:               {} - {}/s",
      human_bytes(self.read as f64),
      human_bytes(self.read as f64 / secs)
    )?;
    writeln!(
      f,
      "write:              {} - {}/s",
      human_bytes(self.write as f64),
      human_bytes(self.write as f64 / secs)
    )?;

    writeln!(
      f,
      "requests/sec:       {}",
      (self.ok as f64 / secs).round() as u64
    )?;

    Ok(())
  }
}
