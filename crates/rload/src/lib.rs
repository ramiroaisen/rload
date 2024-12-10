cfg_if::cfg_if! {
  if #[cfg(all(not(feature = "h1"), not(feature = "h2")))] {
    compile_error!("at least one of feature h1 or feature h2 must be enabled");
  }
}

pub mod cli;
pub mod args;
pub mod io;
#[cfg(feature = "h1")]
pub mod h1;
#[cfg(feature = "h2")]
pub mod h2;

#[cfg(feature = "tls")]
pub mod tls;
