use std::mem::MaybeUninit;
use bytes::BytesMut;
use httparse::{parse_chunk_size, Header, Status};
#[cfg(feature = "monoio")]
use monoio::buf::IoBufMut;

use crate::rt::{Read, ReadExt, Write, WriteExt};

#[cfg(feature = "error-detail")]
use crate::error::ErrorKind;

#[cfg(feature = "status-detail")]
use crate::status::Statuses;

/// The maximum total size of a request head allowed by the h1 parser
const H1_HTTP_MAX_RESPONSE_HEAD_SIZE: usize = 1024 * 128;

/// The maximum headers qty allowed by the h1 parser
const H1_HTTP_MAX_HEADER_QTY: usize = 128;

const CONNECTION: &str = "connection";
const CLOSE: &str = "close";
const CONTENT_LENGTH: &str = "content-length";
const TRANSFER_ENCODING: &str = "transfer-encoding";
const CHUNKED: &str = "chunked";
const COMMA: u8 = b',';

#[cfg(feature = "error-detail")]
type SendError = ErrorKind;

#[cfg(not(feature = "error-detail"))]
type SendError = ();

#[inline(always)]
pub async fn send_request<S: Read + Write + Unpin>(
  stream: &mut S,
  // monoio Write trait requires that the buffer is static
  req_buf: &'static [u8],
  keepalive: bool,
  #[cfg(feature = "status-detail")]
  statuses: &mut Statuses,
  #[cfg(not(feature = "status-detail"))]
  not_ok_status: &mut u64,
  #[cfg(feature = "timeout")]
  timeout: Option<std::time::Duration>,
) -> Result<bool, SendError> {
  
  macro_rules! err {
    ($err:ident) => {{
      #[cfg(feature = "error-detail")]
      {
        Err(SendError::$err)
      }
    
      #[cfg(not(feature = "error-detail"))]
      {
        Err(())
      }
    }}
  }

  let inner = async move {
    #[cfg(feature = "monoio")]
    match stream.write_all(req_buf).await {
      (Ok(_), _) => {}
      (Err(_), _) => return err!(Write),
    };

    #[cfg(not(feature = "monoio"))]
    match stream.write_all(req_buf).await {
      Ok(()) => {}
      Err(_) => return err!(Write),
    };

    #[allow(unused_mut)]
    let mut buf = [MaybeUninit::<u8>::uninit(); H1_HTTP_MAX_RESPONSE_HEAD_SIZE];
        
    cfg_if::cfg_if! {
      if #[cfg(feature = "monoio")] {
        let buf: [u8; H1_HTTP_MAX_RESPONSE_HEAD_SIZE] = unsafe { std::mem::transmute(buf) };
        let mut buf = Box::new(buf);
      } else {
        // Safety: we only read the initialized part of the buf
        let buf = unsafe { assume_init_slice(&mut buf) };
      }  
    };

    let mut filled_len = 0;

    'read: loop {

      #[cfg(feature = "monoio")]
      let n = match stream.read(unsafe { buf.slice_mut_unchecked(filled_len..) }).await {
        (Ok(n), b) => {
          buf = b.into_inner();
          n
        },
        (Err(_), _) => return err!(Read),
      };

      #[cfg(not(feature = "monoio"))]
      // Safety: filled_len can never be greater than buf.len()
      let n = match stream.read(unsafe { buf.get_unchecked_mut(filled_len..) }).await {
        Ok(n) => n,
        Err(_) => return err!(Read),
      };
        
      if n == 0 {
        return err!(Read)
      }

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
      // config.ignore_invalid_headers_in_requests(true)
      // config.allow_multiple_spaces_in_request_line_delimiters(true)

      let head_len = match config.parse_response_with_uninit_headers(&mut res, buf, &mut headers) {
        Ok(httparse::Status::Complete(n)) => n,
        Ok(httparse::Status::Partial) => {
          if filled_len >= H1_HTTP_MAX_RESPONSE_HEAD_SIZE {
            return err!(Parse);
          }
          continue 'read;
        }
        Err(_) => return err!(Parse),
      };
        
      if let Some(status) = res.code {
        #[cfg(feature = "status-detail")]
        unsafe { statuses.record_unchecked(status) };
        
        #[cfg(not(feature = "status-detail"))]
        if !matches!(status, 200..=399) {
          *not_ok_status += 1;
        }
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
            Err(_) => return err!(Parse),
          };

          let len = match str.parse::<u64>() {
            Ok(v) => v,
            Err(_) => return err!(Parse),
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
            Err(_) => return err!(ReadBody)
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
              Err(_) => return err!(ReadBody)
            }

          // not chunked nor content-length
          // but the status code is a "no content" one
          // so we just continue sending requests without further reading
          } else if matches!(res.code, Some(100..=199 | 204 | 205 | 300..=399)) {
            // No content status codes            
            // 100 Continue
            // 101 Switching Protocols
            // 102 Processing
            // 204 No content
            // 205 Reset content
            // 300 Multiple choices
            // 301 Moved permanently
            // 302 Found
            // 303 See other
            // 304 Not modified
            // 305 Use proxy
            // 306 Switch proxy
            // 307 Temporary redirect
            // 308 Permanent redirect
            return Ok(is_keepalive)
          
          
          // no chunked encoding nor content-length, consume the response until the end
          // and dispose the connection, as curl does
          } else {
            match read_to_end(stream).await {
              Ok(()) => return Ok(false),
              Err(_) => return err!(ReadBody)
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
          Err(_) => err!(Timeout)
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

#[cfg(not(feature = "monoio"))]
#[inline(always)]
unsafe fn assume_init_slice<T>(s: &mut [MaybeUninit<T>]) -> &mut [T] {
  let s: *mut [MaybeUninit<T>] = s;
  let s = s as *mut [T];
  &mut *s
}

const SHARED_BUF_LEN: usize = 512 * 1024;
// Safety: we never read the contents of the slice
// and we never create a shared reference for it, only mutable references
static mut SHARED_BUF: [u8; SHARED_BUF_LEN] = [0; SHARED_BUF_LEN];

pub async fn read_to_end<R: Read + Unpin>(r: &mut R) -> Result<(), std::io::Error> {
  loop {

    #[cfg(feature = "monoio")]
    {
      let buf = unsafe { SHARED_BUF.slice_mut_unchecked(..) };
      match r.read(buf).await {
        (Ok(n), _) => {
          if n != 0 {
            continue;
          } else {
            return Ok(())
          }
        },
        (Err(e), _) => return Err(e),
      }
    }
    
    #[cfg(not(feature = "monoio"))]
    {
      let buf = unsafe { &mut SHARED_BUF[..] };
      match r.read(buf).await {
        Ok(n) => {
          if n != 0 {
            continue;
          } else {
            return Ok(())
          }
        },
        Err(e) => return Err(e),
      }
    }
  }
}


#[inline(always)]
async fn read_exact_and_dispose<R: Read + Unpin>(r: &mut R, mut take: u64) -> Result<(), std::io::Error> {
  while take != 0 {
    
    let n = take.min(SHARED_BUF_LEN as u64) as usize;

    #[cfg(feature = "monoio")]
    {
      let slice = unsafe { SHARED_BUF.slice_mut_unchecked(..n) };
      r.read_exact(slice).await.0?;
    };

    #[cfg(not(feature = "monoio"))]
    {
      let slice = unsafe { &mut SHARED_BUF[..n] };
      r.read_exact(slice).await?;
    };

    take -= n as u64
  }

  Ok(())
}

#[inline(always)]
pub async fn consume_chunked_body<R: Read + Unpin>(stream: &mut R, readed: &[u8]) -> Result<(), std::io::Error> {
  
  let mut buf = BytesMut::from(readed);
  let mut first = true;

  'read: loop {

    const MIN_CHUNK_HEAD_LEN: usize = 3; // this account for one digit and \r\n. Example: 0\r\n
    
    #[allow(clippy::bool_comparison)]
    if first == false || buf.len() < MIN_CHUNK_HEAD_LEN { 

      #[cfg(feature = "monoio")]
      match stream.read(buf).await {
        (Ok(n), b) => {
          if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
          }
          buf = b;
        }

        (Err(e), _) => {
          return Err(e);
        }
      }

      #[cfg(not(feature = "monoio"))]
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
        
        #[cfg(feature = "monoio")]
        let n = {
          let (r, b) = stream.read(buf).await;
          buf = b;
          r?
        };

        #[cfg(not(feature = "monoio"))]
        let n = stream.read_buf(&mut buf).await?;

        if n == 0 {
          return Err(std::io::ErrorKind::UnexpectedEof.into());
        }

        continue 'read_chunk;
      }
    }
  }
}