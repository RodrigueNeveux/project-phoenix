//! Phoenix API: an unofficial Discord *user-account* client library.
//!
//! * [`models`]: tolerant serde types for Discord objects + permission math.
//! * [`rest`]: rate-limited HTTP client ([`rest::RestClient`]).
//! * [`gateway`]: websocket session with resume/reconnect ([`gateway::Gateway`]).
//! * [`cache`]: in-memory state store fed from gateway events ([`cache::Cache`]).
//!
//! Not affiliated with or endorsed by Discord.

pub mod cache;
pub mod error;
pub mod gateway;
pub mod models;
pub mod props;
pub mod remote_auth;
pub mod rest;

pub use cache::Cache;
pub use error::{Error, Result};
pub use gateway::{Gateway, GatewayConfig, GatewayEvent, GatewayHandle};
pub use props::ClientProperties;
pub use rest::RestClient;
