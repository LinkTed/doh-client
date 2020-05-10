#[macro_use]
extern crate log;

use tokio::spawn;

mod cmd;
pub use cmd::{get_app, get_listen_config, get_remote_host};

mod cache;
use cache::Cache;

mod config;
pub use config::Config;

mod context;
use context::Context;

mod error;
use error::{Error as DohError, Result as DohResult};

mod remote;
pub use remote::Host as RemoteHost;

mod handler;
use handler::request_handler;

mod listen;
pub use listen::Config as ListenConfig;

use bytes::Bytes;

use dns_message_parser::MAXIMUM_DNS_PACKET_SIZE;

/// Run the `doh-client` with a specific configuration.
pub async fn run(config: Config) -> DohResult<()> {
    let (mut recv, context) = config.into().await?;

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
