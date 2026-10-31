//! Thin HTTP helper around [`reqwest`] for Minutes backend clients.
//!
//! [`HttpClient`] stores a parsed base URL and a shared [`reqwest::Client`].
//! Callers resolve relative paths with [`HttpClient::join_url`], then issue
//! GET or POST requests. Transport failures map to
//! [`HttpClientError::RequestError`]; non-success HTTP statuses map to
//! [`HttpClientError::UnexpectedStatus`].

use std::time::Duration;

use reqwest::header::HeaderMap;
use reqwest::{Client as ReqwestClient, Error as ReqwestError, Response, StatusCode, Url};
use serde::Serialize;
use thiserror::Error;
use url::ParseError;

/// Default per-request timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Errors produced while building URLs or performing HTTP requests.
#[derive(Error, Debug)]
pub enum HttpClientError {
    /// The configured base URL could not be parsed.
    #[error("failed to parse URL: {0}")]
    UrlParseError(ParseError),
    /// A relative path could not be joined onto the base URL.
    #[error("failed to join URL: {0}")]
    UrlJoinError(ParseError),
    /// The underlying [`reqwest::Client`] could not be constructed.
    #[error("failed to build HTTP client: {0}")]
    ClientBuildError(ReqwestError),
    /// The HTTP request failed to send (DNS, connect, timeout, etc.).
    #[error("failed to send request: {0}")]
    RequestError(ReqwestError),
    /// The server responded with a non-success HTTP status.
    #[error("unexpected HTTP status {status}: {body}")]
    UnexpectedStatus {
        /// Response status code.
        status: StatusCode,
        /// Response body text (may be empty or truncated by the server).
        body: String,
    },
}

/// HTTP client bound to a single API base URL.
///
/// Construct with [`HttpClient::new`] or [`HttpClient::with_default_headers`],
/// then use [`HttpClient::get`] / [`HttpClient::post`] / [`HttpClient::post_json`]
/// for requests. Paths passed to [`HttpClient::join_url`] are resolved against
/// the base URL using [`Url::join`].
#[derive(Debug)]
pub struct HttpClient {
    base_url: Url,
    client: ReqwestClient,
}

impl HttpClient {
    /// Creates a client for `base_url` with no default headers.
    ///
    /// # Errors
    ///
    /// Returns [`HttpClientError::UrlParseError`] when `base_url` is not a
    /// valid absolute URL.
    ///
    /// # Examples
    ///
    /// ```
    /// use minutes_client::http::HttpClient;
    ///
    /// let client = HttpClient::new("https://api.example.com/").unwrap();
    /// ```
    pub fn new(base_url: &str) -> Result<Self, HttpClientError> {
        Self::with_default_headers(base_url, HeaderMap::new())
    }

    /// Creates a client for `base_url` that sends `default_headers` on every request.
    ///
    /// # Errors
    ///
    /// Returns [`HttpClientError::UrlParseError`] when `base_url` is invalid, or
    /// [`HttpClientError::ClientBuildError`] when the underlying client cannot
    /// be built.
    pub fn with_default_headers(
        base_url: &str,
        default_headers: HeaderMap,
    ) -> Result<Self, HttpClientError> {
        let base_url = Url::parse(base_url).map_err(HttpClientError::UrlParseError)?;
        let client = ReqwestClient::builder()
            .default_headers(default_headers)
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .map_err(HttpClientError::ClientBuildError)?;

        Ok(Self { base_url, client })
    }

    /// Sends a GET request to `url`.
    ///
    /// # Errors
    ///
    /// Returns [`HttpClientError::RequestError`] on transport failure, or
    /// [`HttpClientError::UnexpectedStatus`] when the response is not 2xx.
    pub async fn get(&self, url: Url) -> Result<Response, HttpClientError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(HttpClientError::RequestError)?;

        handle_response(response).await
    }

    /// Sends a POST request to `url` with a string `body`.
    ///
    /// # Errors
    ///
    /// Returns [`HttpClientError::RequestError`] on transport failure, or
    /// [`HttpClientError::UnexpectedStatus`] when the response is not 2xx.
    pub async fn post(
        &self,
        url: Url,
        body: impl Into<String>,
    ) -> Result<Response, HttpClientError> {
        let response = self
            .client
            .post(url)
            .body(body.into())
            .send()
            .await
            .map_err(HttpClientError::RequestError)?;

        handle_response(response).await
    }

    /// Sends a POST request to `url` with a JSON-encoded `body`.
    ///
    /// Sets `Content-Type: application/json` via [`reqwest::RequestBuilder::json`].
    ///
    /// # Errors
    ///
    /// Returns [`HttpClientError::RequestError`] on transport failure, or
    /// [`HttpClientError::UnexpectedStatus`] when the response is not 2xx.
    pub async fn post_json<T: Serialize + ?Sized>(
        &self,
        url: Url,
        body: &T,
    ) -> Result<Response, HttpClientError> {
        let response = self
            .client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(HttpClientError::RequestError)?;

        handle_response(response).await
    }

    /// Joins `path` onto the client's base URL.
    ///
    /// # Errors
    ///
    /// Returns [`HttpClientError::UrlJoinError`] when the base URL cannot serve
    /// as a join base (for example a `data:` URL) or `path` is otherwise
    /// invalid for joining.
    pub fn join_url(&self, path: &str) -> Result<Url, HttpClientError> {
        self.base_url
            .join(path)
            .map_err(HttpClientError::UrlJoinError)
    }

    /// Joins `path` onto the base URL and appends `query` parameters.
    ///
    /// # Errors
    ///
    /// Returns [`HttpClientError::UrlJoinError`] when joining fails.
    pub fn join_url_with_query<I, K, V>(&self, path: &str, query: I) -> Result<Url, HttpClientError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut url = self.join_url(path)?;
        {
            let mut pairs = url.query_pairs_mut();
            for (key, value) in query {
                pairs.append_pair(key.as_ref(), value.as_ref());
            }
        }
        Ok(url)
    }
}

/// Rejects responses whose status is outside the 2xx success range.
///
/// # Errors
///
/// Returns [`HttpClientError::UnexpectedStatus`] with the status code and body
/// text when the status is not success.
async fn handle_response(response: Response) -> Result<Response, HttpClientError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.unwrap_or_default();
    Err(HttpClientError::UnexpectedStatus { status, body })
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderName, HeaderValue};
    use serde_json::json;
    use wiremock::matchers::{body_string, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn new_accepts_valid_base_url() {
        let client = HttpClient::new("https://api.example.com/v1/").expect("valid URL");
        assert_eq!(client.base_url.as_str(), "https://api.example.com/v1/");
    }

    #[test]
    fn new_rejects_invalid_base_url() {
        let err = HttpClient::new("not a url").expect_err("invalid URL");
        assert!(matches!(err, HttpClientError::UrlParseError(_)));
        assert!(err.to_string().contains("failed to parse URL"));
    }

    #[test]
    fn join_url_resolves_relative_and_absolute_paths() {
        let client = HttpClient::new("https://api.example.com/v1/").expect("valid URL");

        let relative = client.join_url("entries").expect("join relative");
        assert_eq!(relative.as_str(), "https://api.example.com/v1/entries");

        let absolute = client.join_url("/entries").expect("join absolute");
        assert_eq!(absolute.as_str(), "https://api.example.com/entries");
    }

    #[test]
    fn join_url_with_query_appends_parameters() {
        let client = HttpClient::new("https://api.example.com/v1/").expect("valid URL");
        let url = client
            .join_url_with_query("entries", [("start", "2021-01-01"), ("hydrated", "true")])
            .expect("join with query");
        assert_eq!(url.path(), "/v1/entries");
        let query = url.query().expect("query present");
        assert!(query.contains("start=2021-01-01"));
        assert!(query.contains("hydrated=true"));
    }

    #[test]
    fn join_url_rejects_cannot_be_a_base_url() {
        let client = HttpClient::new("data:text/plain,hello").expect("data URL parses");
        let err = client
            .join_url("foo")
            .expect_err("cannot join onto data URL");
        assert!(matches!(err, HttpClientError::UrlJoinError(_)));
        assert!(err.to_string().contains("failed to join URL"));
    }

    #[tokio::test]
    async fn get_returns_response_on_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/entries"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;

        let client = HttpClient::new(&server.uri()).expect("valid mock URL");
        let url = client.join_url("/entries").expect("join");
        let response = client.get(url).await.expect("GET succeeds");
        assert!(response.status().is_success());
        assert_eq!(response.text().await.expect("body"), "ok");
    }

    #[tokio::test]
    async fn get_sends_default_headers() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/entries"))
            .and(header("x-api-key", "secret"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;

        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-api-key"),
            HeaderValue::from_static("secret"),
        );
        let client =
            HttpClient::with_default_headers(&server.uri(), headers).expect("valid mock URL");
        let url = client.join_url("/entries").expect("join");
        let response = client.get(url).await.expect("GET succeeds");
        assert!(response.status().is_success());
    }

    #[tokio::test]
    async fn get_maps_error_status_to_unexpected_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_string("nope"))
            .mount(&server)
            .await;

        let client = HttpClient::new(&server.uri()).expect("valid mock URL");
        let url = client.join_url("/missing").expect("join");
        let err = client.get(url).await.expect_err("404 is an error");
        match &err {
            HttpClientError::UnexpectedStatus { status, body } => {
                assert_eq!(status.as_u16(), 404);
                assert_eq!(body, "nope");
            }
            other => panic!("expected UnexpectedStatus, got {other:?}"),
        }
        assert!(err.to_string().contains("unexpected HTTP status 404"));
    }

    #[tokio::test]
    async fn post_sends_body_and_returns_response_on_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/entries"))
            .and(body_string(r#"{"hours":1}"#))
            .respond_with(ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let client = HttpClient::new(&server.uri()).expect("valid mock URL");
        let url = client.join_url("/entries").expect("join");
        let response = client
            .post(url, r#"{"hours":1}"#)
            .await
            .expect("POST succeeds");
        assert_eq!(response.status().as_u16(), 201);
    }

    #[tokio::test]
    async fn post_json_sends_serialized_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/entries"))
            .and(header("content-type", "application/json"))
            .and(body_string(r#"{"hours":1}"#))
            .respond_with(ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let client = HttpClient::new(&server.uri()).expect("valid mock URL");
        let url = client.join_url("/entries").expect("join");
        let response = client
            .post_json(url, &json!({"hours": 1}))
            .await
            .expect("POST JSON succeeds");
        assert_eq!(response.status().as_u16(), 201);
    }

    #[tokio::test]
    async fn get_with_query_hits_expected_params() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/entries"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .mount(&server)
            .await;

        let client = HttpClient::new(&server.uri()).expect("valid mock URL");
        let url = client
            .join_url_with_query("/entries", [("page", "1")])
            .expect("join");
        let response = client.get(url).await.expect("GET succeeds");
        assert!(response.status().is_success());
    }

    #[tokio::test]
    async fn post_maps_error_status_to_unexpected_status() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/entries"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let client = HttpClient::new(&server.uri()).expect("valid mock URL");
        let url = client.join_url("/entries").expect("join");
        let err = client
            .post(url, "payload")
            .await
            .expect_err("500 is an error");
        match err {
            HttpClientError::UnexpectedStatus { status, .. } => {
                assert_eq!(status.as_u16(), 500);
            }
            other => panic!("expected UnexpectedStatus, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_maps_connection_failure_to_request_error() {
        let client = HttpClient::new("http://127.0.0.1:1").expect("valid URL");
        let url = client.join_url("/unreachable").expect("join");
        let err = client.get(url).await.expect_err("connection should fail");
        assert!(matches!(err, HttpClientError::RequestError(_)));
    }
}
