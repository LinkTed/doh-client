use crate::ListenConfig;
use clap::ArgMatches;
use std::net::{AddrParseError, SocketAddr};

pub fn get_listen_config(arg_matches: &ArgMatches) -> Result<ListenConfig, AddrParseError> {
    let listen_config = if arg_matches.get_flag("listen-activation") {
        ListenConfig::Activation
    } else if let Some(addr) = arg_matches.get_one::<SocketAddr>("listen-addr") {
        ListenConfig::Addr(addr.to_owned())
    } else {
        ListenConfig::Addr("127.0.0.1:53".parse()?)
    };
    Ok(listen_config)
}
