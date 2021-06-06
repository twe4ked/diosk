use log::info;
use mime::Mime;
use thiserror::Error;
use url::Url;

use std::io::prelude::*;
use std::io::{self, BufReader, ErrorKind};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub mod gemtext;
pub mod status_code;
mod tls;

use status_code::StatusCode;

const PORT: u16 = 1965;

#[derive(Debug)]
pub enum Response {
    Body {
        content: Option<String>,
        status_code: StatusCode,
    },
}

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("invalid DNS name")]
    InvalidDnsName(#[from] webpki::InvalidDNSNameError),
    #[error("IO error")]
    IoError(#[from] io::Error),
    #[error("{0}")]
    StatusCodeParseError(#[from] status_code::ParseError),
    #[error("permanent failure: {0} {1}")]
    PermanentFailure(String, String),
    #[error("temporary failure: {0} {1}")]
    TemporaryFailure(String, String),
    #[error("no host")]
    NoHost,
    #[error("redirect loop")]
    RedirectLoop,
}

#[cfg(feature = "debug_content")]
pub fn transaction(_url: &Url) -> Result<Response, TransactionError> {
    Ok(Response::Body {
        content: Some("Foo.\nBar.\nBaz.".to_string()),
        status_code: StatusCode::parse(&"20 text/gemini\r\n").unwrap(),
    })
}

#[cfg(not(feature = "debug_content"))]
pub fn transaction(url: &Url) -> Result<Response, TransactionError> {
    transaction_inner(url, 0)
}

fn transaction_inner(url: &Url, redirect_count: usize) -> Result<Response, TransactionError> {
    let host = url.host_str().ok_or(TransactionError::NoHost)?;

    let mut tls_client = tls::client(&host)?;

    info!("resolving domain");
    let addrs: Vec<_> = format!("{}:{}", &host, &PORT)
        .to_socket_addrs()
        .expect("unable to resolve domain")
        .collect();
    let addr = addrs.first().expect("no domain");

    // C: Opens connection
    // S: Accepts connection
    // C/S: Complete TLS handshake (see section 4)
    // C: Validates server certificate (see 4.2)
    info!("opening socket: {}:{}", &host, &PORT);
    let mut socket = TcpStream::connect_timeout(&addr, Duration::from_secs(4))?;

    info!("opening stream");
    let mut stream = rustls::Stream::new(&mut tls_client, &mut socket);

    // C: Sends request (one CRLF terminated line) (see section 2)
    let request = format!("{}\r\n", url);
    info!("sending request: {}", url);
    stream.write_all(request.as_bytes())?;

    // S: Sends response header (one CRLF terminated line), closes connection under non-success
    //      conditions (see 3.1 and 3.2)
    let mut reader = BufReader::new(stream);

    // Read the header
    let mut header = String::new();
    reader.read_line(&mut header)?;
    let status_code = StatusCode::parse(&header)?;

    // S: Sends response body (text or binary data) (see 3.3)
    // S: Closes connection
    match status_code.clone() {
        StatusCode::Success { code: _, mime_type } => {
            let mut body = Vec::new();
            match reader.read_to_end(&mut body) {
                Ok(_len) => {}
                Err(e) => {
                    match e.kind() {
                        ErrorKind::ConnectionAborted => {
                            // This is expected and should be treated as EOF
                        }
                        _ => panic!("{:?}", e),
                    }
                }
            }

            let mime_type =
                mime_type.unwrap_or_else(|| "text/gemini".parse::<Mime>().expect("infallible"));
            let charset = mime_type.get_param("charset").unwrap_or(mime::UTF_8);

            // C: Handles response (see 3.4)
            match (mime_type.type_(), mime_type.subtype()) {
                (mime::TEXT, name) => match name.as_str() {
                    "gemini" => {
                        let body = encoding::label::encoding_from_whatwg_label(charset.as_str())
                            .expect("unable to find decoder")
                            .decode(&body, encoding::types::DecoderTrap::Replace)
                            .expect("unable to decode");

                        Ok(Response::Body {
                            content: Some(body),
                            status_code,
                        })
                    }
                    _ => todo!("unsupported mime type: {}", mime_type),
                },
                _ => todo!("unsupported mime type: {}", mime_type),
            }
        }
        StatusCode::TemporaryFailure { code, meta } => {
            Err(TransactionError::TemporaryFailure(code, meta))
        }
        StatusCode::PermanentFailure { code, meta } => {
            Err(TransactionError::PermanentFailure(code, meta))
        }
        StatusCode::Redirect {
            code: _,
            url: redirect_url,
        } => {
            // > A user agent SHOULD NOT automatically redirect a request more than 5 times, since
            // > such redirections usually indicate an infinite loop.
            // >    -- RFC-2068 (early HTTP/1.1 specification), section 10.3
            if redirect_count > 5 {
                return Err(TransactionError::RedirectLoop);
            }

            let url = qualify_url(Some(url), &redirect_url.unwrap());
            transaction_inner(&url, redirect_count + 1)
        }
    }
}

pub fn qualify_url(current_url: Option<&Url>, url_or_path: &str) -> Url {
    match Url::parse(&url_or_path) {
        Ok(url) => url,
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            let mut url = current_url.unwrap().clone();
            // If we don't have a URL base, we clear the query/fragment and join
            // on the requested path.
            url.set_query(None);
            url.set_fragment(None);
            url.join(&url_or_path).unwrap()
        }
        e => panic!("{:?}", e),
    }
}
