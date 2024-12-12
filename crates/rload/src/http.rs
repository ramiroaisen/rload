#[derive(Debug, Clone, Copy)]
pub enum Version {
  #[cfg(feature = "h1")]
  Http1,
  #[cfg(feature = "h2")]
  Http2,
}

impl std::fmt::Display for Version {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      #[cfg(feature = "h1")]
      Version::Http1 => write!(f, "http/1"),
      #[cfg(feature = "h2")]
      Version::Http2 => write!(f, "h2"),
    }
  }
}