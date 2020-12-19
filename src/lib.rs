#[macro_use]
extern crate log;

mod cache;
mod cmd;
mod config;
mod context;
mod error;
mod handler;
mod listen;
mod remote;
mod run;

use cache::Cache;
pub use cmd::{get_app, get_listen_config, get_remote_host};
pub use config::Config;
use error::{Error as DohError, Result as DohResult};
pub use listen::Config as ListenConfig;
pub use remote::Host as RemoteHost;
pub use run::run;
