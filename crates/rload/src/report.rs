use std::{fmt::Display, net::SocketAddr, time::Duration};
use human_bytes::human_bytes;
use url::Url;

use crate::error::{Errors, ErrorKind};

#[derive(Debug, Clone)]
pub struct Report {
  pub url: Url,
  pub address: SocketAddr,
  pub http_version: crate::http::Version,
  pub keepalive: bool,

  pub threads: usize,
  pub concurrency: usize,
  pub duration: Duration,
  pub elapsed: Duration,

  #[cfg(feature = "timeout")]
  pub timeout: Option<Duration>,

  pub ok: u64,
  pub err: Errors,
  pub read: u64,
  pub write: u64,
  pub statuses: Vec<(u16, u64)>,

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
      "elapsed:           {}",
      crate::fmt::format_duration(self.elapsed)
    )?;
    writeln!(f, "fulfilled:         {}", self.ok)?;
    {
      let total= self.err.total();
      let Errors {
        connect,
        tls_handshake,
        read_body,
        read,
        write,
        parse,
        h2_handshake,
        h2_ready,
        h2_send,
        h2_recv,
        h2_body,
        timeout,
      } = self.err;
      
      if total == 0 {
        writeln!(f, "errors:            0")?;
      } else {
        writeln!(f, "- Errors")?;
        fn err(f: &mut std::fmt::Formatter<'_>, name: impl Display, count: u64) -> std::fmt::Result { 
          if count != 0 {
            writeln!(f, "  · {: <15}{}", format!("{}:", name), count)?;
          }

          Ok(())
        }

        err(f, "total", total)?;
        err(f, ErrorKind::Connect, connect)?;
        err(f, ErrorKind::TlsHandshake, tls_handshake)?;
        err(f, ErrorKind::ReadBody, read_body)?;
        err(f, ErrorKind::Read, read)?;
        err(f, ErrorKind::Write, write)?;
        err(f, ErrorKind::Parse, parse)?;
        err(f, ErrorKind::H2Handshake, h2_handshake)?;
        err(f, ErrorKind::H2Ready, h2_ready)?;
        err(f, ErrorKind::H2Send, h2_send)?;
        err(f, ErrorKind::H2Recv, h2_recv)?;
        err(f, ErrorKind::H2Body, h2_body)?;
        err(f, ErrorKind::Timeout, timeout)?;
      }
    }

    let has_non_200 = self.statuses.iter().any(|(status, _)| *status != 200);
    if has_non_200 {
      writeln!(f, "- Status codes")?;
      for (status, count) in self.statuses.iter() {
        writeln!(f, "  · {}: {}", status, count)?;
      }
    }

    writeln!(
      f,
      "read:              {} - {}/s",
      human_bytes(self.read as f64),
      human_bytes(self.read as f64 / secs)
    )?;
    writeln!(
      f,
      "write:             {} - {}/s",
      human_bytes(self.write as f64),
      human_bytes(self.write as f64 / secs)
    )?;

    writeln!(
      f,
      "requests/sec:      {}",
      (self.ok as f64 / secs).round() as u64
    )?;

    Ok(())
  }
}