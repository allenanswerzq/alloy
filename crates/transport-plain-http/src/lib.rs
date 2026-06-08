#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

use alloy_transport::{utils::guess_local_url, TransportError, TransportErrorKind};
#[cfg(feature = "reqwest")]
use std::task;
#[cfg(feature = "reqwest")]
use tower::Service;
use url::Url;

#[cfg(feature = "reqwest")]
use alloy_json_rpc::{RequestPacket, Response, ResponsePacket, ResponsePayload, SerializedRequest};
#[cfg(feature = "reqwest")]
use alloy_transport::{TransportFut, TransportResult};
#[cfg(feature = "reqwest")]
use serde_json::Value;

#[cfg(feature = "reqwest")]
pub use reqwest;
pub use url;

/// Plain JSON-over-HTTP transport.
///
/// This adapts RPC client requests to APIs that expect a plain JSON object instead of a JSON-RPC
/// envelope. Request params are sent as the raw JSON request body, and the plain JSON response is
/// returned as the call result.
#[derive(Clone, Debug)]
pub struct PlainHttp<T> {
    client: T,
    url: Url,
}

impl<T> PlainHttp<T> {
    /// Create a new plain HTTP transport with a custom client.
    pub const fn with_client(client: T, url: Url) -> Self {
        Self { client, url }
    }

    /// Set the URL.
    pub fn set_url(&mut self, url: Url) {
        self.url = url;
    }

    /// Set the client.
    pub fn set_client(&mut self, client: T) {
        self.client = client;
    }

    /// Guess whether the URL is local, based on the hostname.
    #[must_use]
    pub fn guess_local(&self) -> bool {
        guess_local_url(&self.url)
    }

    /// Get a reference to the client.
    pub const fn client(&self) -> &T {
        &self.client
    }

    /// Get the URL as a string.
    #[must_use]
    pub fn url(&self) -> &str {
        self.url.as_ref()
    }

    /// Get a reference to the URL.
    pub const fn url_ref(&self) -> &Url {
        &self.url
    }
}

#[cfg(feature = "reqwest")]
impl PlainHttp<reqwest::Client> {
    /// Create a new plain HTTP transport with a default reqwest client.
    #[must_use]
    pub fn new(url: Url) -> Self {
        Self { client: Default::default(), url }
    }

    #[tracing::instrument(name = "plain_rpc_request", skip_all, fields(url = %self.url, method = %request.method()))]
    async fn do_reqwest(self, request: SerializedRequest) -> TransportResult<Response> {
        let id = request.id().clone();
        let headers = request.headers().cloned();
        let body = plain_request_body(&request)?;

        let mut builder = self.client.post(self.url).json(&body);
        if let Some(headers) = headers {
            builder = builder.headers(headers);
        }

        let resp = builder.send().await.map_err(TransportErrorKind::custom)?;
        let status = resp.status();

        tracing::debug!(%status, "received response from server");

        let body = resp.bytes().await.map_err(TransportErrorKind::custom)?;
        if tracing::enabled!(tracing::Level::TRACE) {
            tracing::trace!(body = %String::from_utf8_lossy(&body), "response body");
        } else {
            tracing::debug!(bytes = body.len(), "retrieved response body");
        }

        if !status.is_success() {
            return Err(TransportErrorKind::http_error(
                status.as_u16(),
                String::from_utf8_lossy(&body).into_owned(),
            ));
        }

        let result = serde_json::from_slice(&body)
            .map_err(|err| TransportError::deser_err(err, String::from_utf8_lossy(&body)))?;
        Ok(Response { id, payload: ResponsePayload::Success(result) })
    }
}

#[cfg(feature = "reqwest")]
fn plain_request_body(request: &SerializedRequest) -> TransportResult<Value> {
    let Some(params) = request.params() else {
        return Ok(Value::Null);
    };
    serde_json::from_str(params.get()).map_err(|err| TransportError::deser_err(err, params.get()))
}

#[cfg(feature = "reqwest")]
impl Service<RequestPacket> for PlainHttp<reqwest::Client> {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        let this = self.clone();
        Box::pin(async move {
            match req {
                RequestPacket::Single(request) => {
                    this.do_reqwest(request).await.map(ResponsePacket::Single)
                }
                RequestPacket::Batch(_) => Err(TransportErrorKind::custom_str(
                    "plain HTTP transport does not support batch requests",
                )),
            }
        })
    }
}

/// A [`PlainHttp`] transport using [`reqwest`].
#[cfg(feature = "reqwest")]
pub type ReqwestPlainHttp = PlainHttp<reqwest::Client>;
