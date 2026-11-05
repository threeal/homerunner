//! A command-line interface for deploying and managing persistent self-hosted
//! GitHub Actions runners in Docker.
//!
//! The CLI isn't implemented yet; it only prints the toolkit's status.

#[cfg(not(tarpaulin_include))]
fn main() {
    clap::Command::new("homerunner")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Deploy and manage persistent self-hosted GitHub Actions runners in Docker")
        .get_matches();

    println!("{}", homerunner::status());
}
