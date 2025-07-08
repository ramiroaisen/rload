
use strum::{EnumCount, EnumIter, IntoEnumIterator};
#[derive(Debug, Clone, Copy)]
pub struct Errors([u64; ErrorKind::COUNT]);

#[repr(u8)]
#[derive(Debug, Clone, Copy, EnumIter, EnumCount)]
pub enum ErrorKind {
  Connect = 0,
  TlsHandshake,
  Read,
  ReadBody,
  Write,
  Parse,
  Timeout,
  H2Handshake,
  H2Ready,
  H2Send,
  H2Recv,
  H2Body,
}

impl std::fmt::Display for ErrorKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorKind::Connect => write!(f, "connect"),
      ErrorKind::TlsHandshake => write!(f, "tls-handshake"),
      ErrorKind::Read => write!(f, "read"),
      ErrorKind::ReadBody => write!(f, "read-body"),
      ErrorKind::Write => write!(f, "write"),
      ErrorKind::Parse => write!(f, "parse"),
      ErrorKind::Timeout => write!(f, "timeout"),
      ErrorKind::H2Handshake => write!(f, "h2-handshake"),
      ErrorKind::H2Ready => write!(f, "h2-ready"),
      ErrorKind::H2Send => write!(f, "h2-send"),
      ErrorKind::H2Recv => write!(f, "h2-recv"),
      ErrorKind::H2Body => write!(f, "h2-body"),
    }
  }
}

impl Errors {
  #[inline(always)]
  pub const fn new() -> Self {
    Self([0;ErrorKind::COUNT])
  }

  #[inline(always)]
  pub fn get(&self, item: ErrorKind) -> u64 {
    let index = item as usize;
    // Safety: ErrorKind::COUNT is the length of the array
    unsafe {
      *self.0.get_unchecked(index)
    }
  }

  #[inline(always)]
  pub fn record(&mut self, item: ErrorKind) {
    let index = item as usize;
    // Safety: ErrorKind::COUNT is the length of the array
    unsafe {
      *self.0.get_unchecked_mut(index) += 1;
    }
  }

  /// An iterator over the non-zero-count pairs of (error: ErrorKind, count: u64)
  pub fn iter(&self) -> impl Iterator<Item = (ErrorKind, u64)> + '_ {
    ErrorKind::iter().filter_map(|kind| {
      let count = self.get(kind);
      if count == 0 {
        None
      } else {
        Some((kind, count))
      }
    })
  }

  #[inline(always)]
  pub fn join(&mut self, other: Self) {
    for (i, item) in other.0.into_iter().enumerate() {
      // Safety: both arrays are the same length
      unsafe {
        *self.0.get_unchecked_mut(i) += item;
      }
    }
  }

  pub fn total(self) -> u64 {
    self.0.iter().sum()
  }
}

impl Default for Errors {
  #[inline(always)]
  fn default() -> Self {
    Self::new()
  }
}
