use anyhow::Context;
use axum::{body::Body, response::Response};
use clap::Parser;
use hyper::StatusCode;
use rand::{thread_rng, Rng};
use serde::Deserialize;
// use rand::Rng;
use std::{
  convert::Infallible,
  net::IpAddr,
  str::FromStr,
};

#[derive(Debug, clap::Parser)]
struct Args {
  #[clap(short, long, default_value_t = num_cpus::get(), env = "THREADS")]
  threads: usize,

  #[clap(short, long, default_value_t = IpAddr::from([127, 0, 0, 1]), env = "ADDR")]
  addr: IpAddr,

  #[clap(short, long, default_value_t = 8080, env = "PORT")]
  port: u16,
}

fn main() -> Result<(), anyhow::Error> {
  let args = Args::parse();

  let rt = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(args.threads)
    .enable_all()
    .build()
    .context("failed to build tokio runtime")?;

  rt.block_on(async_main(args))
}

async fn async_main(args: Args) -> Result<(), anyhow::Error> {

  let app = axum::Router::new()
    .route("/", axum::routing::get(root))
    .route("/random-status", axum::routing::get(random_status))
    .route("/full/:unit/:len", axum::routing::get(full))
    .route("/chunked/:unit/:len", axum::routing::get(chunked))
    .route("/echo", axum::routing::post(echo));

  let addr = std::net::SocketAddr::from((args.addr, args.port));

  let tcp = tokio::net::TcpListener::bind(addr)
    .await
    .with_context(|| format!("error binding to {addr}"))?;

  eprintln!("server listening on http://{addr}");

  axum::serve(tcp, app)
    .await
    .context("error serving requests")?;

  Ok(())
}

async fn root() -> Response {
  Response::new(Body::empty())
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
  B,
  KB,
  MB,
}

impl Unit {
  fn multiplier(self) -> usize {
    match self {
      Unit::B => 1,
      Unit::KB => 1024,
      Unit::MB => 1024 * 1024,
    }
  }
}

impl FromStr for Unit {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "b" => Ok(Unit::B),
      "kb" => Ok(Unit::KB),
      "mb" => Ok(Unit::MB),
      _ => anyhow::bail!("invalid unit, must be one of b, kb, mb"),
    }
  }
}

const CHUNK_SIZE: usize = 1024;
#[axum::debug_handler]
async fn chunked(axum::extract::Path((unit, n)): axum::extract::Path<(Unit, usize)>) -> Response {
  let mut remain = n * unit.multiplier();
  let body = Body::from_stream(async_stream::stream! {
    while remain != 0 {
      let size = remain.min(CHUNK_SIZE);
      let chunk = vec![b'0'; size];
      yield Ok::<_, Infallible>(chunk);
      remain -= size;
    }
  });

  Response::new(body)
}

#[axum::debug_handler]
async fn full(axum::extract::Path((unit, n)): axum::extract::Path<(Unit, usize)>) -> Response {
  let body = Body::from(vec![b'0'; n * unit.multiplier()]);
  Response::new(body)
}

async fn echo(req: axum::extract::Request) -> Response {
  let body = req.into_body();
  Response::new(body)
}

async fn random_status() -> Response {
  let status = thread_rng().gen_range(100..=999);
  let mut res = Response::new(Body::empty());
  *res.status_mut() = StatusCode::from_u16(status).unwrap();
  res
}
