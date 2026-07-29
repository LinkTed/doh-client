use crate::config::Config;
use crate::error::Result as DohResult;
use crate::handler::{request_handler, tcp_connection_handler};
use bytes::Bytes;
use dns_message_parser::MAXIMUM_DNS_PACKET_SIZE;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::spawn;

/// Run the `doh-client` with a specific configuration.
pub async fn run(config: Config) -> DohResult<()> {
    let (recv, context) = config.into().await?;
    let local_addr = recv.local_addr()?;
    let context = Arc::new(context);

    match tokio::net::TcpListener::bind(local_addr).await {
        Ok(tcp_listener) => {
            info!("Listening for DNS requests on UDP and TCP address {}", local_addr);
            let tcp_context = context.clone();
            spawn(async move {
                loop {
                    match tcp_listener.accept().await {
                        Ok((stream, _)) => {
                            let c = tcp_context.clone();
                            spawn(async move {
                                if let Err(e) = tcp_connection_handler(stream, c).await {
                                    debug!("TCP connection closed: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("TCP accept error: {}", e);
                        }
                    }
                }
            });
        }
        Err(e) => {
            error!("Failed to bind TCP listener on {}: {}", local_addr, e);
        }
    }

    let mut buffer: [u8; MAXIMUM_DNS_PACKET_SIZE] = [0; MAXIMUM_DNS_PACKET_SIZE];
    loop {
        let (n, addr) = recv.recv_from(&mut buffer[..]).await?;
        let msg = Bytes::copy_from_slice(&buffer[..n]);
        debug!("Receive UDP packet: {:?}", msg);
        let c = context.clone();
        spawn(async move {
            if let Err(e) = request_handler(msg, addr, c.as_ref()).await {
                error!("Could not handle request: {}", e);
            }
        });
    }
}
