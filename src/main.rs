use doh_client::{get_command, run, Config};
use env_logger::Builder;
use log::error;

#[tokio::main]
async fn main() {
    let mut builder = Builder::from_default_env();
    builder.format_timestamp(None).init();

    let matches = get_command().get_matches();
    let config = Config::try_from(matches).await;
    match config {
        Ok(config) => {
            if let Err(e) = run(config).await {
                error!("doh-client stopped: {}", e);
            }
        }
        Err(e) => error!("Could not start doh-client: {}", e),
    }
}
