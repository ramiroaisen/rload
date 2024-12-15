
use strum::{EnumCount, EnumIter};
#[derive(Debug, Clone, Copy)]
pub struct Errors([u64; ErrorKind::COUNT]);

#[repr(u8)]
#[derive(Debug, Clone, Copy, EnumIter, EnumCount)]
pub enum ErrorKind {
  Connect = 0,
  TlsHandshake = 1,
  Read = 2,
  ReadBody = 3,
  Write = 4,
  Parse = 5,
  Timeout = 6,
  H2Handshake = 7,
  H2Ready = 8,
  H2Send = 9,
  H2Recv = 10,
  H2Body = 11,
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
    debug_assert!(index < self.0.len());
    unsafe {
      *self.0.get_unchecked(index)
    }
  }

  #[inline(always)]
  pub fn record(&mut self, item: ErrorKind) {
    let index = item as usize;
    debug_assert!(index < self.0.len());
    unsafe {
      *self.0.get_unchecked_mut(index) += 1;
    }
  }

  #[inline(always)]
  pub fn join(&mut self, other: Self) {
    for (i, item) in other.0.into_iter().enumerate() {
      let index = i;
      debug_assert!(index < self.0.len());
      unsafe {
        *self.0.get_unchecked_mut(index) += item;
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
