use crate::config::Host;
use gotham::anyhow::Error as AError;
use gotham::handler::IntoResponse;
use gotham::helpers::http::response::create_response;
use gotham::hyper::StatusCode;
use gotham::hyper::{body::Body, Response};
use gotham::state::State;
use isahc::{Request, RequestExt};
use mime::Mime;
use sentry_types::Dsn;
use serde_json::Value;

use log::*;

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

/**
 * Represent a sentry envelope
 */
#[derive(Debug)]
pub struct SentryEnvelope {
    pub raw_body: Vec<u8>,
    pub dsn: Dsn,
}

/**
 * A body parsing error
 */
#[derive(Debug)]
pub enum BodyError {
    InvalidNumberOfLines,
    InvalidHeaderJson(serde_json::Error),
    MissingDsnKeyInHeader,
    InvalidDsnValue,
    InvalidProjectId,
    EmptyBody,
}

impl Display for BodyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BodyError::InvalidNumberOfLines => {
                f.write_str("Invalid number of lines in request body. Must have at least 2 lines (header and at least one item).")
            }
            BodyError::MissingDsnKeyInHeader => {
                f.write_str("The dsn key is missing from the header")
            }
            BodyError::InvalidHeaderJson(e) => {
                f.write_fmt(format_args!("Failed to parse header json : {}", e))
            }
            BodyError::InvalidProjectId => f.write_str("Unauthorized project ID"),
            BodyError::InvalidDsnValue => f.write_str("Failed to parse dsn value"),
            BodyError::EmptyBody => f.write_str("Empty request body"),
        }
    }
}

impl Error for BodyError {}

impl IntoResponse for BodyError {
    fn into_response(self, state: &State) -> Response<Body> {
        warn!("{}", self);
        let mime = "application/json".parse::<Mime>().unwrap();
        create_response(state, StatusCode::BAD_REQUEST, mime, format!("{}", self))
    }
}

impl SentryEnvelope {
    /**
     * Returns true if this envelope is for an host that we are allowed to forward requests to
     */
    pub fn dsn_host_is_valid(&self, host: &[Host]) -> bool {
        let envelope_host = self.dsn.host().to_string();
        host.iter()
            .any(|x| x.0 == envelope_host)
    }

    /**
     * Forward this envelope to the destination sentry relay
     */
    pub async fn forward(&self) -> Result<(), AError> {
        let uri = self.dsn.envelope_api_url().to_string() + "?sentry_key=" + self.dsn.public_key();
        let request = Request::builder()
            .uri(uri)
            .header("Content-type", "application/x-sentry-envelope")
            .method("POST")
            .body(self.raw_body.clone())?;
        info!(
            "Sending HTTP {} {} - body length={}",
            request.method(),
            request.uri(),
            self.raw_body.len()
        );
        match request.send_async().await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /**
     * Attempt to parse bytes into an envelope
     * Supports envelopes with varying numbers of lines (session replays, etc.)
     */
    pub fn try_new_from_body(body: Vec<u8>) -> Result<SentryEnvelope, AError> {
        if body.is_empty() {
            return Err(AError::new(BodyError::EmptyBody));
        }

        // Find the first newline to extract the header
        let header_end = body.iter().position(|&b| b == b'\n')
            .ok_or_else(|| AError::new(BodyError::InvalidNumberOfLines))?;
        
        // Parse the header (first line)
        let header_bytes = &body[..header_end];
        let header_str = std::str::from_utf8(header_bytes)
            .map_err(|_| AError::new(BodyError::InvalidHeaderJson(
                serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Header contains invalid UTF-8"
                ))
            )))?;
        
        let header: Value = serde_json::from_str(header_str)
            .map_err(|e| BodyError::InvalidHeaderJson(e))?;
        
        if let Some(dsn) = header.get("dsn") {
            if let Some(dsn_str) = dsn.as_str() {
                let dsn = Dsn::from_str(dsn_str)?;
                Ok(SentryEnvelope {
                    dsn,
                    raw_body: body,
                })
            } else {
                Err(AError::new(BodyError::InvalidDsnValue))
            }
        } else {
            Err(AError::new(BodyError::MissingDsnKeyInHeader))
        }
    }
}
