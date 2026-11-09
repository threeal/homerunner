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
        .subcommand(
            clap::Command::new("list")
                .about("List the runners, with their repositories and container states"),
        )
        .subcommand(
            clap::Command::new("remove")
                .about("Stop a runner and remove its container and volume")
                .arg(clap::arg!(<NAME> "The name of the runner")),
        )
        .subcommand(
            clap::Command::new("update")
                .about("Update the runners, or only the one named, to the latest image")
                .arg(clap::arg!([NAME] "The name of the runner")),
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
        Some(("list", _)) => {
            homerunner::list().map(|runners| print!("{}", homerunner::format_runners(&runners)))
        }
        Some(("remove", matches)) => {
            let name = matches.get_one::<String>("NAME").expect("NAME is required");
            homerunner::remove(name).map(|()| println!("removed runner {name}"))
        }
        Some(("update", matches)) => {
            let name = matches.get_one::<String>("NAME").map(String::as_str);
            homerunner::update(name).map(|names| {
                for name in names {
                    println!("updated runner {name}");
                }
            })
        }
        _ => unreachable!("a subcommand is required"),
    };

    if let Err(err) = result {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
