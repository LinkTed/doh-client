use super::{response_handler, Config, Host};
use crate::{DohError, DohResult};
use base64::{encode_config, URL_SAFE_NO_PAD};
use bytes::Bytes;
use dns_message_parser::Dns;
use h2::client::SendRequest;
use http::Request;
use rustls::ClientConfig;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

pub(crate) struct Session {
    config: Config,
    host: Host,
    connection_id: u32,
    send_request: Option<SendRequest<Bytes>>,
}

impl Session {
    pub(crate) fn new(
        host: Host,
        domain: String,
        client_config: Arc<ClientConfig>,
        uri: String,
        retries: u32,
        post: bool,
    ) -> Session {
        let config = Config::new(domain, client_config, uri, retries, post);
        Session {
            config,
            host,
            connection_id: 0,
            send_request: None,
        }
    }

    async fn connect(&mut self) -> DohResult<()> {
        if self.send_request.is_some() {
            return Ok(());
        }
        let config = &self.config;
        let client_config = &config.client_config;
        let domain = &config.domain.as_str();
        for i in 0..config.retries {
            info!("Try to connect to {}: {}", self.host, i + 1);
            match self.host.connect(client_config, domain).await {
                Ok(send_request) => {
                    info!("Connected to {} at {}", domain, self.host);
                    self.send_request.replace(send_request);
                    self.connection_id = self.connection_id.wrapping_add(1);
                    return Ok(());
                }
                Err(e) => {
                    error!("Could not connect to {} at {}: {}", domain, self.host, e);
                }
            }
        }
        Err(DohError::CouldNotConnectServer)
    }

    pub(crate) fn disconnect(&mut self, connection_id: u32) {
        if self.connection_id == connection_id {
            debug!("Disconnect connetion to server");
            self.send_request.take();
        }
    }

    pub(crate) async fn send_request(
        &mut self,
        data: Bytes,
    ) -> DohResult<(
        impl Future<Output = DohResult<(Dns, Option<Duration>)>>,
        u32,
    )> {
        let config = &self.config;
        let post = config.post;

        let request = if post {
            Request::builder()
                .method("POST")
                .uri(config.uri.clone())
                .header("accept", "application/dns-message")
                .header("content-type", "application/dns-message")
                .header("content-length", data.len().to_string())
                .body(())
                .unwrap()
        } else {
            let uri = format!(
                "{}?dns={}",
                config.uri,
                encode_config(&data[..], URL_SAFE_NO_PAD)
            );
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("accept", "application/dns-message")
                .body(())
                .unwrap()
        };

        self.connect().await?;

        debug!("Send HTTP2 request to server: {:?}", request);
        if let Some(send_request) = self.send_request.as_mut() {
            let (response, mut request) = send_request.send_request(request, false)?;
            if post {
                debug!("Send HTTP2 body: {:?}", data);
                request.send_data(data, true)?;
            }
            Ok((response_handler(response), self.connection_id))
        } else {
            Err(DohError::IsNotConnected)
        }
    }

    pub(crate) async fn start_request(
        &mut self,
        dns_request: &mut Dns,
    ) -> DohResult<(
        impl Future<Output = DohResult<(Dns, Option<Duration>)>>,
        u32,
    )> {
        self.connect().await?;
        let id = dns_request.id;
        dns_request.id = 0;
        let bytes = dns_request.encode()?;
        debug!("Send DNS request to server: {}", dns_request);
        let data = bytes.freeze();
        dns_request.id = id;

        match self.send_request(data).await {
            Ok(r) => Ok(r),
            Err(e) => {
                self.send_request.take();
                Err(e)
            }
        }
    }
}
