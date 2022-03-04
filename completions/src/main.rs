use clap_complete::{generate_to, Shell};
use doh_client::get_command;

fn main() {
    let mut command = get_command();
    generate_to(Shell::Zsh, &mut command, "doh-client", ".").unwrap();
    generate_to(Shell::PowerShell, &mut command, "doh-client", ".").unwrap();
    generate_to(Shell::Bash, &mut command, "doh-client", ".").unwrap();
    generate_to(Shell::Elvish, &mut command, "doh-client", ".").unwrap();
    generate_to(Shell::Fish, &mut command, "doh-client", ".").unwrap();
}
