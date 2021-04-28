use log::info;
use mime::Mime;
use rustls::{Certificate, ClientConfig, DangerousClientConfig, RootCertStore};
use thiserror::Error;
use url::Url;

use std::io::prelude::*;
use std::io::{self, BufReader, ErrorKind};
use std::net::TcpStream;
use std::sync::Arc;

pub mod gemtext;

pub struct NoCertificateVerification {}

impl rustls::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _roots: &RootCertStore,
        _presented_certs: &[Certificate],
        _dns_name: webpki::DNSNameRef<'_>,
        _ocsp_response: &[u8],
    ) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
        // TODO: Implement TOFU
        // https://gemini.circumlunar.space/docs/tls-tutorial.gmi
        Ok(rustls::ServerCertVerified::assertion())
    }
}

fn new_config() -> ClientConfig {
    let mut cfg = ClientConfig::new();

    let mut dangerous_config = DangerousClientConfig { cfg: &mut cfg };
    dangerous_config.set_certificate_verifier(Arc::new(NoCertificateVerification {}));

    cfg
}

#[derive(Debug, Clone)]
pub enum StatusCode {
    Success {
        code: String,
        mime_type: Option<Mime>,
    },
    TemporaryFailure {
        code: String,
    },
    Redirect {
        code: String,
        url: Option<String>,
    },
}

impl StatusCode {
    // <STATUS><SPACE><META><CR><LF>
    fn parse(input: &str) -> StatusCode {
        info!("header: {}", input.trim());

        let mut parts = input.split(' ');

        let code = parts.next().unwrap()[0..2].to_string();

        match &code[0..1] {
            "2" => {
                // The <META> line is a MIME media type which applies to the response body
                let rest: String = parts.collect();
                let rest = rest.trim();

                let mime_type: mime::Mime = rest.parse().expect("unable to parse mime type");

                StatusCode::Success {
                    code,
                    mime_type: Some(mime_type),
                }
            }
            "3" => {
                // <META> is a new URL for the requested resource
                let url = parts.next().map(|s| s.to_owned());
                StatusCode::Redirect { code, url }
            }
            "4" => {
                // The contents of <META> may provide additional information on the failure, and should be
                // displayed to human users
                StatusCode::TemporaryFailure { code }
            }
            s => panic!("invalid status code: {}", s),
        }
    }

    pub fn code(&self) -> String {
        match self {
            StatusCode::Success { code, .. } => code,
            StatusCode::TemporaryFailure { code } => code,
            StatusCode::Redirect { code, .. } => code,
        }
        .clone()
    }
}

const PORT: u16 = 1965;

pub enum Response {
    Body {
        content: Option<String>,
        status_code: StatusCode,
    },
    RedirectLoop(Option<String>),
}

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("see: https://github.com/briansmith/webpki/issues/90")]
    BadDer,
}

pub fn transaction(url: &Url, redirect_count: usize) -> Result<Response, TransactionError> {
    let host = url.host_str().expect("no host");

    let config = new_config();
    let dns_name = webpki::DNSNameRef::try_from_ascii_str(&host).unwrap();
    let mut tls_client = rustls::ClientSession::new(&Arc::new(config), dns_name);

    // C: Opens connection
    // S: Accepts connection
    // C/S: Complete TLS handshake (see section 4)
    // C: Validates server certificate (see 4.2)
    let mut socket = TcpStream::connect(&format!("{}:{}", host, PORT)).unwrap();

    let mut stream = rustls::Stream::new(&mut tls_client, &mut socket);

    // C: Sends request (one CRLF terminated line) (see section 2)
    let request = format!("{}\r\n", url);

    info!("sending request: {}", url);

    match stream.write(request.as_bytes()) {
        Ok(_) => {}
        Err(e) => match e.kind() {
            io::ErrorKind::InvalidData => {
                // Custom { kind: InvalidData, error: WebPKIError(BadDER) }
                return Err(TransactionError::BadDer);
            }
            _ => panic!("unable to write to stream: {}", e),
        },
    }

    // S: Sends response header (one CRLF terminated line), closes connection under non-success
    //      conditions (see 3.1 and 3.2)
    let mut reader = BufReader::new(stream);

    // Read the header
    let mut header = String::new();
    reader.read_line(&mut header).unwrap();
    let status_code = StatusCode::parse(&header);

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

            let mime_type = mime_type.unwrap();
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
        StatusCode::TemporaryFailure { .. } => todo!(),
        StatusCode::Redirect { code: _, url } => {
            // > A user agent SHOULD NOT automatically redirect a request more than 5 times, since
            // > such redirections usually indicate an infinite loop.
            // >    -- RFC-2068 (early HTTP/1.1 specification), section 10.3
            if redirect_count > 5 {
                return Ok(Response::RedirectLoop(url));
            }

            let url =
                Url::parse(&url.expect("missing redirect URL")).expect("invalid redirect URL");
            transaction(&url, redirect_count + 1)
        }
    }
}
