use log::info;
use mime::Mime;
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum StatusCode {
    Success {
        code: String,
        mime_type: Option<Mime>,
    },
    TemporaryFailure {
        code: String,
        meta: String,
    },
    Redirect {
        code: String,
        url: Option<String>,
    },
    PermanentFailure {
        code: String,
        meta: String,
    },
}

#[derive(Error, Debug)]
#[error("status code parse error: {0}")]
pub struct ParseError(String);

impl StatusCode {
    // <STATUS><SPACE><META><CR><LF>
    pub(super) fn parse(input: &str) -> Result<StatusCode, ParseError> {
        info!("header: {}", input.trim());

        let mut parts = input.splitn(2, ' ');

        let code: String = parts.next().expect("infallible").chars().take(2).collect();

        match (code.chars().nth(0), code.chars().nth(1)) {
            (Some('2'), Some(_)) => {
                // The <META> line is a MIME media type which applies to the response body
                let rest: String = parts.collect();
                let rest = rest.trim();

                let mime_type: mime::Mime = rest
                    .parse()
                    .unwrap_or_else(|_| "text/gemini; charset=utf-8".parse().expect("infallible"));

                Ok(StatusCode::Success {
                    code,
                    mime_type: Some(mime_type),
                })
            }
            (Some('3'), Some(_)) => {
                // <META> is a new URL for the requested resource
                let url = parts.next().map(|s| s.to_owned());
                Ok(StatusCode::Redirect { code, url })
            }
            (Some('4'), Some(_)) => {
                // The contents of <META> may provide additional information on the failure, and
                // should be displayed to human users
                let meta: String = parts.collect();
                let meta = meta.trim().to_string();
                Ok(StatusCode::TemporaryFailure { code, meta })
            }
            (Some('5'), Some(_)) => {
                // The contents of <META> may provide additional information on the failure, and
                // should be displayed to human users
                let meta: String = parts.collect();
                let meta = meta.trim().to_string();
                Ok(StatusCode::PermanentFailure { code, meta })
            }
            (_, _) => Err(ParseError(input.lines().next().unwrap().to_string())),
        }
    }

    pub fn code(&self) -> String {
        match self {
            StatusCode::Success { code, .. } => code,
            StatusCode::TemporaryFailure { code, .. } => code,
            StatusCode::Redirect { code, .. } => code,
            StatusCode::PermanentFailure { code, .. } => code,
        }
        .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_code_parse() {
        assert!(StatusCode::parse(&"20 text/plain\r\n").is_ok());
        assert!(StatusCode::parse(&"20").is_ok());
        assert!(StatusCode::parse(&"30").is_ok());
        assert!(StatusCode::parse(&"50").is_ok());

        assert!(StatusCode::parse(&"").is_err());
    }
}
