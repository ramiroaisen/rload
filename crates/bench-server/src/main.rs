use std::net::IpAddr;
use anyhow::Context;
use axum::{body::Body, response::Response};
use clap::Parser;
use hyper::StatusCode;
use rand::Rng;

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

  let rt = tokio::runtime::Builder
    ::new_multi_thread()
    .worker_threads(args.threads)
    .enable_all()
    .build()
    .context("failed to build tokio runtime")?;

  rt.block_on(async_main(args))
}

async fn async_main(args: Args) -> Result<(), anyhow::Error> {
  let app = axum::Router::new()
    .route("/", axum::routing::get(root)) 
    .route("/random-status", axum::routing::get(random_status)); 

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


async fn random_status() -> Response {
  let status = rand::thread_rng().gen_range(200u16..=999);
  let status = StatusCode::from_u16(status).unwrap();
  let mut res = Response::new(Body::empty());
  *res.status_mut() = status;
  res
}