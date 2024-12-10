use anyhow::Context;
use http::Uri;
use rustls::pki_types::ServerName;
use std::{
  net::{SocketAddr, ToSocketAddrs},
  sync::Arc,
  time::Duration,
};
use url::Url;

use crate::{cli::Args, tls};

#[derive(Clone)]
pub struct Tls<'a> {
  pub connector: tokio_rustls::TlsConnector,
  pub server_name: ServerName<'a>,
}

#[derive(Debug, Clone, Copy)]
pub enum Scheme {
  Http,
  Https,
}

#[derive(Clone, Copy)]
pub enum Request<'a> {
  H1 {
    // this is the pre-encoded request to write directly to the socket
    buf: &'a [u8],
  },
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
  pub no_keepalive: bool,
  pub request: Request<'a>,
  pub tls: Option<&'a Tls<'a>>,
  pub duration: Duration,
}

impl RunConfig<'static> {
  pub fn from_args(args: Args) -> Result<Self, anyhow::Error> {
    let Args {
      url,
      threads,
      concurrency,
      no_keepalive,
      h2,
      duration,
    } = args;

    if threads == 0 {
      anyhow::bail!("threads option must be greater than 0");
    }

    if concurrency == 0 {
      anyhow::bail!("concurrency option must be greater than 0");
    }

    if duration.as_secs_f64() < 0.000_000_001 {
      anyhow::bail!("duration option must be equal or greater than 1ns");
    }

    let url: &'static _ = Box::leak(Box::new(url.parse::<Url>().context("error parsing url")?));

    let host: &'static _ = url
      .host_str()
      .context("invalid url, missing host")?
      .to_string()
      .leak();

    let (scheme, tls) = match url.scheme() {
      "http" => (Scheme::Http, None),
      "https" => {
        let client_config = if h2 {
          tls::h2_client_config()
        } else {
          tls::h1_client_config()
        };

        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
        let server_name = ServerName::try_from(host).context("invalid server name in host")?;

        let tls: &'static _ = Box::leak(Box::new(Tls {
          connector,
          server_name,
        }));

        (Scheme::Https, Some(tls))
      }
      other => anyhow::bail!("invalid scheme {other}, must be http or https"),
    };

    let port = url.port_or_known_default().unwrap();

    let addr = format!("{}:{}", host, port)
      .to_socket_addrs()
      .with_context(|| format!("error resolving address for {url}"))?
      .next()
      .with_context(|| format!("socket addresses for {url} resolved to empty list"))?;

    let request = {
      match scheme {
        Scheme::Http => {
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

            if no_keepalive {
              req_lines.push(String::from("connection: close"));
            }

            req_lines.push(String::from("\r\n"));
            req_lines.join("\r\n").leak().as_bytes()
          };

          Request::H1 { buf }
        }
        Scheme::Https => {
          let req: &'static _ = {
            let mut req = http::Request::new(());
            *req.uri_mut() = Uri::from_static(url.to_string().leak());
            Box::leak(Box::new(req))
          };

          Request::H2 { req }
        }
      }
    };

    let config = RunConfig::<'static> {
      url,
      addr,
      threads,
      concurrency,
      no_keepalive,
      request,
      tls,
      duration,
    };

    Ok(config)
  }
}
