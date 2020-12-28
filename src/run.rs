use crate::config::Config;
use crate::context::Context;
use crate::error::Result as DohResult;
use crate::handler::request_handler;
use bytes::Bytes;
use dns_message_parser::MAXIMUM_DNS_PACKET_SIZE;
use tokio::spawn;

/// Run the `doh-client` with a specific configuration.
pub async fn run(config: Config) -> DohResult<()> {
    let (recv, context) = config.into().await?;

    let context: &'static Context = Box::leak(Box::new(context));

    let mut buffer: [u8; MAXIMUM_DNS_PACKET_SIZE] = [0; MAXIMUM_DNS_PACKET_SIZE];
    loop {
        let (n, addr) = recv.recv_from(&mut buffer[..]).await?;
        let msg = Bytes::copy_from_slice(&buffer[..n]);
        debug!("Receive UDP packet: {:?}", msg);
        spawn(async move {
            if let Err(e) = request_handler(msg, addr, context).await {
                error!("Could not handle request: {}", e);
            }
        });
    }
}
