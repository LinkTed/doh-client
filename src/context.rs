use crate::remote::Session as RemoteSession;
use crate::Cache;
use dns_message_parser::question::Question;
use dns_message_parser::Dns;
use futures::lock::Mutex;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;

/// The context object for a running instance.
pub struct Context {
    pub(crate) sender: Arc<UdpSocket>,
    pub(crate) remote_session: Mutex<RemoteSession>,
    pub(crate) cache: Option<Mutex<Cache<Question, Dns>>>,
    pub(super) cache_fallback: bool,
    pub(crate) timeout: Duration,
}

impl Context {
    /// Create a new `doh_client::Context` object.
    pub(super) fn new(
        cache: Option<Mutex<Cache<Question, Dns>>>,
        cache_fallback: bool,
        timeout: u64,
        remote_session: RemoteSession,
        sender: Arc<UdpSocket>,
    ) -> Context {
        Context {
            sender,
            remote_session: Mutex::new(remote_session),
            cache,
            cache_fallback,
            timeout: Duration::from_secs(timeout),
        }
    }
}
