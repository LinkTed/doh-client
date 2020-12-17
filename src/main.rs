#[macro_use]
extern crate log;
#[macro_use]
extern crate clap;

use env_logger::Builder;

use doh_client::{get_app, get_listen_config, get_remote_host, run, Config};

#[tokio::main]
async fn main() {
    let matches = get_app().get_matches();

    let mut builder = Builder::from_default_env();
    builder.format_timestamp(None).init();

    let listen_config = match get_listen_config(&matches) {
        Ok(listen_config) => listen_config,
        Err(e) => {
            error!("Could not get listen config: {}", e);
            return;
        }
    };
    let remote_host = match get_remote_host(&matches).await {
        Ok(remote_host) => remote_host,
        Err(e) => {
            error!("Could not get remote host: {:?}", e);
            return;
        }
    };
    let domain = matches.value_of("domain").unwrap();
    let cafile = matches.value_of("cafile");
    let path = matches.value_of("path").unwrap();
    let retries: u32 = value_t!(matches, "retries", u32).unwrap_or(3);
    let timeout: u64 = value_t!(matches, "timeout", u64).unwrap_or(2);
    let post: bool = !matches.is_present("get");
    let cache_size: usize = value_t!(matches, "cache-size", usize).unwrap_or(1024);
    let cache_fallback: bool = matches.is_present("cache-fallback");
    let result = Config::new(
        listen_config,
        remote_host,
        domain,
        cafile,
        path,
        retries,
        timeout,
        post,
        cache_size,
        cache_fallback,
    );
    match result {
        Ok(config) => {
            if let Err(e) = run(config).await {
                error!("doh-client stopped: {}", e);
            }
        }
        Err(e) => error!("Could not start doh-client: {}", e),
    }
}
