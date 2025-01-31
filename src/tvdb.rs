use std::fmt::Display;

use const_format::concatcp;
use reqwest::{
    blocking::Client,
    header::CONTENT_TYPE, StatusCode,
};
use serde::Deserialize;

use crate::media::MediaType;

const API_BASE_URL: &str = "https://api4.thetvdb.com/v4";

/// Client for the TVDB API, implements only the needed functionality for this software
pub struct TvdbClient {
    api_key: String,
    client: Client,
    token: Option<String>,
}

impl TvdbClient {
    pub fn new<S>(api_key: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
            token: None,
        }
    }

    pub fn login(&mut self) -> Result<(), TvdbError> {
        let res = self
            .client
            .post(concatcp!(API_BASE_URL, "/login"))
            .header(CONTENT_TYPE, "application/json")
            .body(format!("{{\"apikey\": \"{}\"}}", self.api_key))
            .send()
            .map_err(TvdbError::RequestError)?;

        if res.status() != StatusCode::OK {
            return Err(TvdbError::HttpError(res.status()));
        }

        let text = res.text().map_err(TvdbError::RequestError)?;
        let json: ApiReply<LoginReply> =
            serde_json::from_str(&text).map_err(TvdbError::ParseError)?;

        self.token = Some(json.data.token);

        Ok(())
    }

    pub fn search(&self, name: &str, media_type: MediaType) -> Result<SearchReply, TvdbError> {
        let res = self
            .client
            .get(concatcp!(API_BASE_URL, "/search"))
            .query(&[("q", name), ("type", media_type.into())])
            .bearer_auth(self.token()?)
            .send()
            .map_err(TvdbError::RequestError)?;

        if res.status() != StatusCode::OK {
            return Err(TvdbError::HttpError(res.status()));
        }

        let text = res.text().map_err(TvdbError::RequestError)?;
        let json: ApiReply<SearchReply> =
            serde_json::from_str(&text).map_err(TvdbError::ParseError)?;

        Ok(json.data)
    }

    fn token(&self) -> Result<&str, TvdbError> {
        self.token
            .as_ref()
            .map(|s| s.as_str())
            .ok_or(TvdbError::Unauthenticated)
    }
}

#[derive(Debug)]
pub enum TvdbError {
    Unauthenticated,
    RequestError(reqwest::Error),
    ParseError(serde_json::Error),
    HttpError(StatusCode),
}

impl Display for TvdbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TvdbError::Unauthenticated => write!(f, "Unauthenticated"),
            TvdbError::RequestError(error) => write!(f, "Request error: {}", error),
            TvdbError::ParseError(error) => write!(f, "Parse error: {}", error),
            TvdbError::HttpError(status_code) => write!(f, "HTTP error: {}", status_code),
        }
    }
}

#[derive(Deserialize)]
struct ApiReply<T> {
    status: String,
    data: T,
}

#[derive(Deserialize)]
struct LoginReply {
    token: String,
}

pub type SearchReply = Vec<SearchResult>;

#[derive(Deserialize)]
pub struct SearchResult {
    pub name: String,
}
