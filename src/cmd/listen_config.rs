use crate::ListenConfig;

use clap::ArgMatches;

use std::net::AddrParseError;

pub fn get_listen_config(arg_matches: &ArgMatches) -> Result<ListenConfig, AddrParseError> {
    let listen_config = if arg_matches.is_present("listen-activation") {
        ListenConfig::Activation
    } else if arg_matches.is_present("listen-addr") {
        let addr = arg_matches.value_of("listen-addr").unwrap().parse()?;
        ListenConfig::Addr(addr)
    } else {
        ListenConfig::Addr("127.0.0.1:53".parse()?)
    };
    Ok(listen_config)
}
