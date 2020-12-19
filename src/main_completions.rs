use clap::Shell::*;
use doh_client::get_app;

fn main() {
    let mut app = get_app();
    app.gen_completions("doh-client", Zsh, "completions/");
    app.gen_completions("doh-client", PowerShell, "completions/");
    app.gen_completions("doh-client", Bash, "completions/");
    app.gen_completions("doh-client", Elvish, "completions/");
    app.gen_completions("doh-client", Fish, "completions/");
}
