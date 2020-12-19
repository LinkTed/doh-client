use rustls::ClientConfig;
use std::sync::Arc;

pub(super) struct Config {
    pub(super) domain: String,
    pub(super) client_config: Arc<ClientConfig>,
    pub(super) uri: String,
    pub(super) retries: u32,
    pub(super) post: bool,
}

impl Config {
    pub(super) fn new(
        domain: String,
        client_config: Arc<ClientConfig>,
        uri: String,
        retries: u32,
        post: bool,
    ) -> Config {
        Config {
            domain,
            client_config,
            uri,
            retries,
            post,
        }
    }
}
