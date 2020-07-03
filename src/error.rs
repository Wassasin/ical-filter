use actix_web::{client::SendRequestError, HttpResponse, ResponseError};
use awc::error::PayloadError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum Error {
    UpstreamFailure,
    AuthenticationFailure,
    Inconsistency,
}

impl core::convert::From<SendRequestError> for Error {
    fn from(_: SendRequestError) -> Self {
        Error::UpstreamFailure
    }
}

impl core::convert::From<PayloadError> for Error {
    fn from(_: PayloadError) -> Self {
        Error::UpstreamFailure
    }
}

impl core::convert::From<ical::parser::ParserError> for Error {
    fn from(_: ical::parser::ParserError) -> Self {
        Error::Inconsistency
    }
}

pub type Result<T> = core::result::Result<T, Error>;

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

impl ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        let mut response = match self {
            Error::UpstreamFailure => HttpResponse::ServiceUnavailable(),
            Error::AuthenticationFailure => HttpResponse::InternalServerError(),
            Error::Inconsistency => HttpResponse::InternalServerError(),
        };

        response.json(self)
    }
}
