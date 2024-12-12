use std::mem::MaybeUninit;

use bytes::BytesMut;
use httparse::{parse_chunk_size, Header, Status};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::{error::ErrorKind, status::Statuses};

/// The maximum total size of a request head allowed by the h1 parser
const H1_HTTP_MAX_RESPONSE_HEAD_SIZE: usize = 1024 * 64;

/// The maximum headers qty allowed by the h1 parser
const H1_HTTP_MAX_HEADER_QTY: usize = 128;

const CONNECTION: &str = "connection";
const CLOSE: &str = "close";
const CONTENT_LENGTH: &str = "content-length";
const TRANSFER_ENCODING: &str = "transfer-encoding";
const CHUNKED: &str = "chunked";
const COMMA: u8 = b',';

#[inline(always)]
pub async fn send_request<S: AsyncRead + AsyncWrite + Unpin>(
  stream: &mut S,
  req_buf: &[u8],
  keepalive: bool,
  statuses: &mut Statuses,
  #[cfg(feature = "timeout")]
  timeout: Option<std::time::Duration>,
) -> Result<bool, ErrorKind> {
  let inner = async move {
    match stream.write_all(req_buf).await {
      Ok(()) => {}
      Err(_) => return Err(ErrorKind::Write),
    };

    // Safety: we only read the initialized part of the buf
    let mut slice = [MaybeUninit::<u8>::uninit(); H1_HTTP_MAX_RESPONSE_HEAD_SIZE];
    let buf = unsafe { assume_init_slice(&mut slice) };

    let mut filled_len = 0;

    'read: loop {
      // Safety: filled_len can never be greater than buf.len()
      let n = match stream.read(unsafe { buf.get_unchecked_mut(filled_len..) }).await {
        Ok(n) => {
          if n == 0 {
            return Err(ErrorKind::Read)
          } 
          n
        }
        Err(_) => return Err(ErrorKind::Read),
      };
      
      filled_len += n;
      // we override the buf here to avoid use it after this line
      // now the buf is only the filled part
      let buf = &buf[..filled_len];
      // we gain a little performance here, httparse will take care of initializing the headers
      let mut headers = [MaybeUninit::<Header<'_>>::uninit(); H1_HTTP_MAX_HEADER_QTY];
      let mut res = httparse::Response::new(&mut []);

      let mut config = httparse::ParserConfig::default();
        
      // we set the most permissive config
      config.allow_spaces_after_header_name_in_responses(true);
      config.allow_obsolete_multiline_headers_in_responses(true);
      config.allow_space_before_first_header_name(true);
      config.ignore_invalid_headers_in_responses(true);
      
      // this are for request only
      // config.allow_multiple_spaces_in_request_line_delimiters(true)

      let head_len = match config.parse_response_with_uninit_headers(&mut res, buf, &mut headers) {
        Ok(httparse::Status::Complete(n)) => n,
        Ok(httparse::Status::Partial) => {
          if filled_len == H1_HTTP_MAX_RESPONSE_HEAD_SIZE {
            return Err(ErrorKind::Parse);
          }
          continue 'read;
        }
        Err(_) => return Err(ErrorKind::Parse),
      };
        
            
      if let Some(status) = res.code {
        // Safety: httparse parses status codes as three digit numbers, so the max possible value is 999
        unsafe { statuses.record_unchecked(status) };
      }

      let is_keepalive = 'k: {
        if !keepalive || res.version != Some(1) {
          // if disabled keepalive in arguments or server http version is http/1.0 we are not using keepalive
          false
        } else {
          for h in res.headers.iter() {
            if h.name.eq_ignore_ascii_case(CONNECTION) {
              // if "connection" contains "close" we are not in keepalive
              break 'k !header_contains(h.value, CLOSE);
            }
          }

          // if no connection header we default to keepalive, as in http/1.1 default
          true
        }
      };

      let content_length = 'content_length: {
        for h in res.headers.iter() {
          if !h.name.eq_ignore_ascii_case(CONTENT_LENGTH) {
            continue;
          }

          let str = match std::str::from_utf8(h.value) {
            Ok(str) => str,
            Err(_) => return Err(ErrorKind::Parse),
          };

          let len = match str.parse::<u64>() {
            Ok(v) => v,
            Err(_) => return Err(ErrorKind::Parse),
          };

          break 'content_length Some(len);
        }

        None
      };

      match content_length {
        // identity encoding with content length
        Some(content_length) => {
          let to_read = (head_len as u64 + content_length).saturating_sub(buf.len() as u64);

          if to_read == 0 {
            return Ok(is_keepalive);
          }

          match read_exact_and_dispose(stream, to_read).await {
            Ok(()) => return Ok(is_keepalive),
            Err(_) => return Err(ErrorKind::ReadBody)
          }
        }

        None => {
          let is_chunked = 'c: {
            for h in res.headers.iter() {
              if h.name.eq_ignore_ascii_case(TRANSFER_ENCODING) {
                // if "Transfer-Encoding" contains "chunked" item, then the transfer-encoding is chunked 
                break 'c header_contains(h.value, CHUNKED);
              }
            }
            // if no transfer-encoding header we default to not chunked, as http spec
            false
          };

          // chunked encoding
          if is_chunked {

            // Safety: head_len could never overflow the buf len
            match consume_chunked_body(stream, unsafe { buf.get_unchecked(head_len..) }).await {
              Ok(()) => return Ok(is_keepalive),
              Err(_) => return Err(ErrorKind::ReadBody)
            }

          // no chunked encoding nor content-length, consume the response until the end
          // and dispose the connection, as curl does
          } else {
            match read_to_end(stream).await {
              Ok(()) => return Ok(false),
              Err(_) => return Err(ErrorKind::ReadBody)
            }
          }
        }
      }
    }
  };

  #[cfg(not(feature = "timeout"))]
  {
    inner.await
  }

  #[cfg(feature = "timeout")]
  {
    match timeout {
      Some(timeout) => {
        // note that pingora timeouts will ceil to the next 10ms
        match pingora_timeout::timeout(timeout, inner).await {
          Ok(r) => r,
          Err(_) => Err(ErrorKind::Timeout)
        }
      }

      None => inner.await
    }
  }
}

/// Returns true if a list formatted header value contains the needle
/// Eg: "Transfer-Ecoding: chunked,gzip" contains "chunked" and "gzip" 
#[inline(always)]
fn header_contains(v: &[u8], needle: &str) -> bool {
  for item in v.split(|c| *c == COMMA) {
    if item.trim_ascii().eq_ignore_ascii_case(needle.as_bytes()) {
      return true
    }
  }

  false
}

#[inline(always)]
unsafe fn assume_init_slice<T>(s: &mut [MaybeUninit<T>]) -> &mut [T] {
  let s: *mut [MaybeUninit<T>] = s;
  let s = s as *mut [T];
  &mut *s
}

const SHARED_BUF_LEN: usize = 512 * 1024;
// Safety: we never read the contents of the slice
static mut SHARED_BUF: [u8; SHARED_BUF_LEN] = [0; SHARED_BUF_LEN];

pub async fn read_to_end<R: AsyncRead + Unpin>(r: &mut R) -> Result<(), std::io::Error> {
  loop {
    match r.read(unsafe { &mut SHARED_BUF[..] }).await {
      Err(e) => return Err(e),
      Ok(0) => return Ok(()),
      Ok(_) => continue,
    }
  }
}


#[inline(always)]
async fn read_exact_and_dispose<R: AsyncRead + Unpin>(r: &mut R, mut take: u64) -> Result<(), std::io::Error> {
  while take != 0 {
    let max = take.min(SHARED_BUF_LEN as u64) as usize;
    let slice = unsafe { &mut SHARED_BUF[..max] };
    r.read_exact(slice).await?;
    take -= max as u64
  }

  Ok(())
}

#[inline(always)]
pub async fn consume_chunked_body<R: AsyncRead + Unpin>(stream: &mut R, readed: &[u8]) -> Result<(), std::io::Error> {
  
  let mut buf = BytesMut::from(readed);
  let mut first = true;

  'read: loop {

    const MIN_CHUNK_HEAD_LEN: usize = 3; // this account for one digit and \r\n. Example: 0\r\n
    
    #[allow(clippy::bool_comparison)]
    if first == false || buf.len() < MIN_CHUNK_HEAD_LEN { 
      match stream.read_buf(&mut buf).await {
        Ok(n) => {
          if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
          }
        }

        Err(e) => {
          return Err(e);
        }
      }
    }

    first = false;

    'parse: loop {

      let (consumed, size) = match parse_chunk_size(&buf) {
        // correct parsing of chunk size
        Ok(Status::Complete((consumed, size))) => (consumed, size),


        Ok(Status::Partial) => {
          // chunk size incomplete, continue reading
          continue 'read;
        }

        Err(_) => {
          // invalid chunk size
          return Err(std::io::ErrorKind::InvalidData.into())
        }
      };

      // last chunk
      if size == 0 {
        let expected = consumed + 2; // +2 for the \r\n
        
        if buf.len() == expected {
          return Ok(())
        } 
        
        if buf.len() < expected {
          let take = expected - buf.len();
          read_exact_and_dispose( stream, take as u64).await?;
          return Ok(())
        }
        
        return Err(std::io::ErrorKind::Other.into());
      }

      // not last chunk
      let until = consumed as u64 + size + 2; // +2 is for the \r\n at the end of the chunk

      'read_chunk: loop { 

        if buf.len() as u64 == until {
          buf.clear();
          continue 'read;
        } 
        
        if buf.len() as u64 > until {
          let _ = buf.split_to(until as usize);
          continue 'parse;
        }
        
        stream.read_buf(&mut buf).await?;
        continue 'read_chunk;
      }
    }
  }
}

#[inline(always)]
pub async fn consume_chunked_body_alt1<R: AsyncRead + Unpin>(stream: &mut R, readed: &[u8]) -> Result<(), std::io::Error> {
  
  let mut buf = BytesMut::from(readed);
  let mut first = true;

  'read: loop {

    const MIN_CHUNK_HEAD_LEN: usize = 3; // this account for one digit and \r\n. Example: 0\r\n
    
    #[allow(clippy::bool_comparison)]
    if first == false || buf.len() < MIN_CHUNK_HEAD_LEN { 
      match stream.read_buf(&mut buf).await {
        Ok(n) => {
          if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
          }
        }

        Err(e) => {
          return Err(e);
        }
      }
    }

    first = false;

    'parse: loop {

      let (consumed, size) = match parse_chunk_size(&buf) {
        // correct parsing of chunk size
        Ok(Status::Complete((consumed, size))) => (consumed, size),


        Ok(Status::Partial) => {
          // chunk size incomplete, continue reading
          continue 'read;
        }

        Err(_) => {
          // invalid chunk size
          return Err(std::io::ErrorKind::InvalidData.into())
        }
      };

      // last chunk
      if size == 0 {
        let expected = consumed + 2; // +2 for the \r\n
        
        if buf.len() == expected {
          return Ok(())
        } 
        
        if buf.len() < expected {
          let take = expected - buf.len();
          read_exact_and_dispose( stream, take as u64).await?;
          return Ok(())
        }
        
        return Err(std::io::ErrorKind::Other.into());
      }

      // not last chunk
      let until = consumed as u64 + size + 2; // +2 is for the \r\n at the end of the chunk
      
      #[allow(clippy::comparison_chain)]
      if buf.len() as u64 == until {
        buf.clear();
        continue 'read;
      } 
        
      if buf.len() as u64 > until {
        let _ = buf.split_to(until as usize);
        continue 'parse;
      }

      let mut to_consume = until - buf.len() as u64;
      buf.clear();
        
      'consume: loop {
        
        let n = stream.read_buf(&mut buf).await?;
        
        if n == 0 {
          return Err(std::io::ErrorKind::UnexpectedEof.into());
        }

        if n as u64 == to_consume {
          buf.clear();
          continue 'read;
        }

        if (n as u64) < to_consume {
          to_consume -= n as u64;
          buf.clear();
          continue 'consume;
        }

        let _ = buf.split_to(to_consume as usize);
        continue 'parse;
      }
    }
  }
}

#[inline(always)]
pub async fn consume_chunked_body_alt2<R: AsyncRead + Unpin>(stream: &mut R, readed: &[u8]) -> Result<(), std::io::Error> {

  let mut buf = BytesMut::from(readed);
  let mut first = true;

  'read: loop {
    
    // const MAX_CHUNK_SIZE_BYTES: u64 = 16 + 2; // 16 is u64::MAX in hex format +2 for the \r\n

    #[allow(clippy::bool_comparison)]
    if buf.is_empty() || first == false { 
      match stream.read_buf(&mut buf).await {
        Ok(n) => {
          if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
          }
        }

        Err(e) => {
          return Err(e);
        }
      }
    }

    first = false;

    'parse: loop {

      let (consumed, size) = match parse_chunk_size(&buf) {
        // correct parsing of chunk size
        Ok(Status::Complete((consumed, size))) => (consumed, size),


        Ok(Status::Partial) => {
          // chunk size incomplete, continue reading
          continue 'read;
        }

        Err(_) => {
          // invalid chunk size
          return Err(std::io::ErrorKind::InvalidData.into())
        }
      };

      // last chunk
      if size == 0 {
        let expected = consumed + 2; // +2 for the \r\n
        #[allow(clippy::comparison_chain)]
        if buf.len() == expected {
          return Ok(())
        }
        
        if buf.len() < expected {
          let take = expected - buf.len();
          read_exact_and_dispose( stream, take as u64).await?;
          return Ok(())
        }
        
        return Err(std::io::ErrorKind::Other.into());
      }

      // not last chunk
      let until = consumed as u64 + size + 2; // +2 is for the \r\n at the end of the chunk

      #[allow(clippy::comparison_chain)]
        if buf.len() as u64 == until {
        buf.clear();
        continue 'read;
      } 
      
      if buf.len() as u64 > until {
        let _ = buf.split_to(until as usize);
        continue 'parse;
      } 
      
      let skip = until - buf.len() as u64;
      read_exact_and_dispose( stream, skip).await?;
      buf.clear();
      continue 'read;
    }
  }
}