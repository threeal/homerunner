//! A command-line interface for deploying and managing persistent self-hosted
//! GitHub Actions runners in Docker.

#[cfg(not(tarpaulin_include))]
fn main() {
    let matches = clap::Command::new("homerunner")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Deploy and manage persistent self-hosted GitHub Actions runners in Docker")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            clap::Command::new("add")
                .about("Register a new self-hosted runner for a repository and start it")
                .arg(clap::arg!(<URL> "The URL of the repository"))
                .arg(
                    clap::arg!(--token <TOKEN> "The registration token of the runner")
                        .required(true),
                ),
        )
        .get_matches();

    let result = match matches.subcommand() {
        Some(("add", matches)) => {
            let url = matches.get_one::<String>("URL").expect("URL is required");
            let token = matches
                .get_one::<String>("token")
                .expect("token is required");
            homerunner::add(url, token).map(|name| println!("added runner {name}"))
        }
        _ => unreachable!("a subcommand is required"),
    };

    if let Err(err) = result {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
