use anyhow::Context;
use std::{
  net::{SocketAddr, ToSocketAddrs},
  time::Duration,
};
use url::Url;
use clap::Parser;

#[cfg(feature = "h2")]
use http::Uri;

#[cfg(feature = "tls")]
use rustls::pki_types::ServerName;
#[cfg(feature = "tls")]
use std::sync::Arc;

fn parse_duration(s: &str) -> Result<Duration, String> {
  let re = regex_static::static_regex!(r"^([0-9]+(?:\.[0-9]+)?)(ns|us|ms|s|m|h|d)$");
  if let Some(captures) = re.captures(s.trim()) {
    let float = captures.get(1).unwrap().as_str().parse::<f64>().unwrap();
    let unit = captures.get(2).unwrap().as_str();

    let multiplier = match unit {
      "ns" => 0.000_000_001,
      "us" => 0.000_001,
      "ms" => 0.001,
      "s" => 1.0,
      "m" => 60.0,
      "h" => 60.0 * 60.0,
      "d" => 60.0 * 60.0 * 24.0,
      _ => unreachable!(),
    };

    let dur = Duration::from_secs_f64(float * multiplier);

    Ok(dur)
  } else {
    Err(String::from("invalid duration, duration must be in the format of interger or float followed by a unit that must be one of ns, us, ms, s, m, h, or d"))
  }
}

#[derive(Debug, Parser)]
#[command(
  disable_version_flag = true,
  disable_help_flag = true,
  version = crate::build::CLAP_LONG_VERSION,
  arg_required_else_help = true
)]
pub struct Args {
  /// the URL to benchmark
  #[arg(env = "URL")]
  pub url: String,

  /// Number of connections to keep open
  #[arg(short, long, default_value_t = 10, env = "CONCURRENCY")]
  pub concurrency: usize,

  /// Duration for the test 
  #[arg(
    short,
    long,
    default_value = "10s",
    env = "DURATION",
    value_parser = parse_duration
  )]
  pub duration: Duration,

  /// Number of threads to use
  #[arg(short, long, default_value_t = 2, env = "THREADS")]
  pub threads: usize,

  /// Add headers to the request
  #[arg(short = 'H', long, value_parser, env = "HEADER")]
  pub header: Vec<String>,
  
  
  /// Timeout for each request 
  #[cfg(feature = "timeout")]
  #[arg(
    short = 'u',
    long,
    env = "TIMEOUT",
    value_parser = parse_duration
  )]
  pub timeout: Option<Duration>,

  /// Disable keepalive
  #[arg(short = 'r', long, visible_alias = "dk", default_value_t = false, env = "DISABLE_KEEPALIVE")]
  pub disable_keepalive: bool,

  /// Enable latency measurement and reporting
  #[cfg(feature = "latency")]
  #[arg(short = 'l', long, default_value_t = false, env = "LATENCY")]
  pub latency: bool,

  /// Use h2 protocol
  #[cfg(all(feature = "h1", feature = "h2"))]
  #[arg(short = '2', long, default_value_t = false, env = "H2")]
  pub h2: bool,

  /// Print version information
  #[arg(short = 'v', short_alias = 'V', long, action = clap::builder::ArgAction::Version)]
  pub version: (),

  /// Print this help message
  #[arg(short = 'h', long, action = clap::builder::ArgAction::Help)]
  pub help: (),
}


#[cfg(feature = "tls")]
#[derive(Clone)]
pub struct Tls<'a> {
  pub connector: tokio_rustls::TlsConnector,
  pub server_name: ServerName<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum Scheme {
  #[cfg(feature = "h1")]
  Http,
  #[cfg(feature = "h2")]
  Https,
}

#[derive(Clone, Copy)]
pub enum Request<'a> {
  #[cfg(feature = "h1")]
  H1 {
    // this is the pre-encoded request to write directly to the socket
    buf: &'a [u8],
  },
  #[cfg(feature = "h2")]
  H2 {
    req: &'a http::Request<()>,
  },
}

#[derive(Clone, Copy)]
pub struct RunConfig<'a> {
  pub url: &'a Url,
  pub addr: SocketAddr,
  pub threads: usize,
  pub concurrency: usize,
  pub disable_keepalive: bool,
  #[cfg(feature = "timeout")] 
  pub timeout: Option<Duration>,
  #[cfg(feature = "latency")]
  pub latency: bool,
  pub request: Request<'a>,
  #[cfg(feature = "tls")]
  pub tls: Option<&'a Tls<'a>>,
  pub duration: Duration,
}

impl RunConfig<'static> {
  pub fn from_args(args: Args) -> Result<Self, anyhow::Error> {
    let Args {
      url,
      threads,
      concurrency,
      disable_keepalive,
      #[cfg(feature = "timeout")]
      timeout,
      #[cfg(feature = "latency")]
      latency,
      #[cfg(all(feature = "h1", feature = "h2"))]
      h2,
      duration,
      header,
      version: _,
      help: _,
    } = args;

    if threads == 0 {
      anyhow::bail!("threads option must be greater than 0");
    }

    if concurrency == 0 {
      anyhow::bail!("concurrency option must be greater than 0");
    }

    if duration.as_nanos() == 0 {
      anyhow::bail!("duration option must be equal or greater than 1ns");
    }

    let url: &'static _ = Box::leak(Box::new(url.parse::<Url>().context("error parsing url")?));

    let host: &'static _ = url
      .host_str()
      .context("invalid url, missing host")?
      .to_string()
      .leak();

    #[cfg(feature = "tls")]
    let tls = match url.scheme() {
      "http" => None,
      
      "https" => {
        cfg_if::cfg_if! {
          if #[cfg(all(feature = "h1", feature = "h2"))] {
            let client_config = if h2 {
              crate::tls::h2_client_config()
            } else {
              crate::tls::h1_client_config()
            };
          } else if #[cfg(feature = "h1")] {
            let client_config = crate::tls::h1_client_config();
          } else if #[cfg(feature = "h2")] {
            let client_config = crate::tls::h2_client_config();
          } else {
            std::compile_error!("at least one of feature h1 or feature h2 must be enabled");
          }
        }

        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
        let server_name = ServerName::try_from(host).context("invalid server name in host")?;

        let tls: &'static _ = Box::leak(Box::new(Tls {
          connector,
          server_name,
        }));

        Some(tls)
      }

      other => anyhow::bail!("invalid scheme {other}, must be http or https"),
    };

    #[cfg(not(feature = "tls"))]
    {
      match url.scheme() {
        "http" => {},

        "https" => anyhow::bail!("feature tls must be enabled at compile time to use https urls"),

        other => anyhow::bail!("invalid scheme {other}, must be http or https (https requires feature=tls to be enabled at compile time, not enabled)"),
      }
    };

    let port = url.port_or_known_default().unwrap();

    let addr = format!("{}:{}", host, port)
      .to_socket_addrs()
      .with_context(|| format!("error resolving address for {url}"))?
      .next()
      .with_context(|| format!("socket addresses for {url} resolved to empty list"))?;

    #[cfg(feature = "h1")]
    macro_rules! h1_req {
      () => {{
        let buf: &'static _ = {
          let mut req_lines = vec![
            format!(
              "GET {}{} HTTP/1.1",
              url.path(),
              match url.query() {
                Some(query) => format!("?{query}"),
                None => String::new(),
              }
            ),
            format!("host: {}", host),
            "content-length: 0".into(),
          ];

          for h in header {
            let (k, v) = h.split_once(':').context("invalid header format, must be key:value")?;
            let hk = http::header::HeaderName::from_bytes(k.trim().as_bytes())
              .with_context(|| format!("invalid header name {k}"))?;
              
            let k = hk.as_str();
            
            let hv = http::header::HeaderValue::from_str(v.trim())
              .with_context(|| format!("invalid header value {v}"))?;
              
            let v = hv.to_str()
              .with_context(|| format!("invalid header value {v}, only utf-8 is supported"))?;
           
            req_lines.push(format!("{k}: {v}"));
          }

          if disable_keepalive {
            req_lines.push(String::from("connection: close"));
          }

          req_lines.push(String::from("\r\n"));
          req_lines.join("\r\n").leak().as_bytes()
        };

        buf
      }};
    }

    #[cfg(feature = "h2")]
    macro_rules! h2_req {
      () => {{
        let req: &'static _ = {
          let mut req = http::Request::new(());
          *req.uri_mut() = Uri::from_static(url.to_string().leak());
          
          for h in header {
            let (k, v) = h.split_once(':').context("invalid header format, must be key:value")?;
            let hk = http::header::HeaderName::from_bytes(k.trim().as_bytes())
              .with_context(|| format!("invalid header name {k}"))?;
            let hv = http::header::HeaderValue::from_str(v.trim())
              .with_context(|| format!("invalid header value {v}"))?;
            req.headers_mut().append(hk, hv);
          }

          Box::leak(Box::new(req))
        };
        req
      }}
    }

    cfg_if::cfg_if! {
      if #[cfg(all(feature = "h1", feature = "h2"))] {
        let request = {
          match h2 {
            false => Request::H1 { buf: h1_req!() },
            true => Request::H2 { req: h2_req!() },
          }
        };
      } else if #[cfg(feature = "h1")] {
        let request = Request::H1 { buf: h1_req!() };
      } else if #[cfg(feature = "h2")] {
        let request = Request::H2 { req: h2_req!() };
      } else {
        std::compile_error!("at least one of feature=h1 or feature=h2 must be enabled");
      }
    };

    let config = RunConfig::<'static> {
      url,
      addr,
      threads,
      concurrency,
      disable_keepalive,
      #[cfg(feature = "timeout")]
      timeout,
      #[cfg(feature = "latency")]
      latency,
      request,
      #[cfg(feature = "tls")]
      tls,
      duration,
    };

    Ok(config)
  }
}
