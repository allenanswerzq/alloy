#![allow(dead_code)]
#![allow(missing_docs)]

#[cfg(feature = "reqwest")]
mod http;

#[cfg(feature = "plain-http")]
mod plain;

#[cfg(feature = "pubsub")]
mod ws;

#[cfg(feature = "pubsub")]
mod ipc;
