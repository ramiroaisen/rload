#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
  Connect,
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

#[derive(Debug, Clone, Copy)]
pub struct Errors {
  pub connect: u64,
  pub tls_handshake: u64,
  pub read: u64,
  pub read_body: u64,
  pub write: u64,
  pub parse: u64,
  pub h2_handshake: u64,
  pub h2_ready: u64,
  pub h2_send: u64,
  pub h2_recv: u64,
  pub h2_body: u64,
  pub timeout: u64,
}

impl Errors {
  #[inline(always)]
  pub const fn new() -> Self {
    Self {
      connect: 0,
      tls_handshake: 0,
      read_body: 0,
      read: 0,
      write: 0,
      parse: 0,
      h2_handshake: 0,
      h2_ready: 0,
      h2_send: 0,
      h2_recv: 0,
      h2_body: 0,
      timeout: 0,
    }
  }

  #[inline(always)]
  pub fn record(&mut self, item: ErrorKind) {
    match item {
      ErrorKind::Connect => self.connect += 1,
      ErrorKind::TlsHandshake => self.tls_handshake += 1,
      ErrorKind::Read => self.read += 1,
      ErrorKind::ReadBody => self.read_body += 1,
      ErrorKind::Write => self.write += 1,
      ErrorKind::Parse => self.parse += 1,
      ErrorKind::Timeout => self.timeout += 1,
      ErrorKind::H2Handshake => self.h2_handshake += 1,
      ErrorKind::H2Ready => self.h2_ready += 1,
      ErrorKind::H2Send => self.h2_send += 1,
      ErrorKind::H2Recv => self.h2_recv += 1,
      ErrorKind::H2Body => self.h2_body += 1,
    }
  }

  #[inline(always)]
  pub fn join(&mut self, other: Self) {
    let Self {
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
    } = other;

    self.connect += connect;
    self.tls_handshake += tls_handshake;
    self.read_body += read_body;
    self.read += read;
    self.write += write;
    self.parse += parse;
    self.h2_handshake += h2_handshake;
    self.h2_ready += h2_ready;
    self.h2_send += h2_send;
    self.h2_recv += h2_recv;    
    self.h2_body += h2_body;
    self.timeout += timeout;
  }

  pub fn total(self) -> u64 {
    let Self {
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
    } = self;

    connect
      + tls_handshake
      + read_body
      + read
      + write
      + parse
      + h2_handshake
      + h2_ready
      + h2_send
      + h2_recv
      + h2_body 
      + timeout
  }
}

impl Default for Errors {
  #[inline(always)]
  fn default() -> Self {
    Self::new()
  }
}



