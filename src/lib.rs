//! A toolkit for deploying and managing persistent self-hosted GitHub Actions
//! runners in Docker.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::process::Command;

/// The image every runner container runs, built from `images/ubuntu-24.04`.
const IMAGE: &str = "ghcr.io/threeal/homerunner:ubuntu-24.04";

/// The label on each runner's volume holding the URL of its repository, which
/// marks the volume as a runner's.
const URL_LABEL: &str = "homerunner.url";

/// A runner managed by homerunner.
#[derive(Debug, PartialEq, Eq)]
pub struct Runner {
    /// The name of the runner, its volume, and its container.
    pub name: String,
    /// The URL of the repository the runner is registered to.
    pub url: String,
    /// The state of the runner's container, such as `running` or `exited`, or
    /// `missing` if it has no container.
    pub state: String,
}

/// Returns every runner managed by homerunner, sorted by name.
///
/// Runners are found by the label on their volumes, so containers and volumes
/// that homerunner didn't create are left out.
///
/// # Errors
///
/// Returns an error if any `docker` call fails.
#[cfg(not(tarpaulin_include))]
pub fn list() -> Result<Vec<Runner>, String> {
    let volumes = docker(&[
        "volume",
        "ls",
        "--filter",
        &format!("label={URL_LABEL}"),
        "--format",
        &format!("{{{{.Name}}}}\t{{{{.Label \"{URL_LABEL}\"}}}}"),
    ])
    .map_err(|err| format!("failed to list volumes: {err}"))?;
    let containers = docker(&["ps", "--all", "--format", "{{.Names}}\t{{.State}}"])
        .map_err(|err| format!("failed to list containers: {err}"))?;
    Ok(parse_runners(&volumes, &containers))
}

/// Returns the runners in `volumes`, lines of a volume's name and URL label,
/// each with the state of the container of the same name in `containers`,
/// lines of a container's name and state, sorted by name.
fn parse_runners(volumes: &str, containers: &str) -> Vec<Runner> {
    let states: HashMap<&str, &str> = containers
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    let mut runners: Vec<Runner> = volumes
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(name, url)| Runner {
            name: name.to_string(),
            url: url.to_string(),
            state: states.get(name).unwrap_or(&"missing").to_string(),
        })
        .collect();
    runners.sort_by(|a, b| a.name.cmp(&b.name));
    runners
}

/// Formats `runners` as a table with one line per runner, holding its name,
/// URL, and state in aligned columns.
#[must_use]
pub fn format_runners(runners: &[Runner]) -> String {
    let name_width = runners.iter().map(|r| r.name.len()).max().unwrap_or(0);
    let url_width = runners.iter().map(|r| r.url.len()).max().unwrap_or(0);
    runners.iter().fold(String::new(), |mut table, r| {
        let _ = writeln!(
            table,
            "{:name_width$}  {:url_width$}  {}",
            r.name, r.url, r.state
        );
        table
    })
}

/// Registers a new self-hosted runner for the repository at `url` and starts
/// it, returning the runner's name.
///
/// The runner is named `<repo>-<N>`, using the lowest `N` not already taken by
/// a Docker volume. Its volume of the same name holds the runner's home
/// directory, and its container of the same name restarts unless stopped,
/// refreshing the apt package index each time it starts.
/// Each step prints a line to standard output before it runs.
/// If any step before starting the container fails, the container and volume
/// created so far are removed.
///
/// # Errors
///
/// Returns an error if `url` has no repository name, or if any `docker` call
/// fails.
#[cfg(not(tarpaulin_include))]
pub fn add(url: &str, token: &str) -> Result<String, String> {
    let repo = repo_name(url)?;
    let volumes = docker(&["volume", "ls", "--format", "{{.Name}}"])
        .map_err(|err| format!("failed to list volumes: {err}"))?;
    let name = next_runner_name(repo, volumes.lines());

    println!("creating volume {name}");
    docker(&[
        "volume",
        "create",
        "--label",
        &format!("{URL_LABEL}={url}"),
        &name,
    ])
    .map_err(|err| format!("failed to create the runner volume: {err}"))?;

    if let Err(err) = create_and_register(&name, url, token) {
        println!("removing container and volume {name}");
        let _ = docker(&["rm", "--force", &name]);
        let _ = docker(&["volume", "rm", &name]);
        return Err(err);
    }

    println!("starting container {name}");
    docker(&["start", &name]).map_err(|err| format!("failed to start the runner: {err}"))?;
    Ok(name)
}

/// Creates the container of the runner `name` without starting it, then
/// registers the runner with GitHub in a throwaway container.
#[cfg(not(tarpaulin_include))]
fn create_and_register(name: &str, url: &str, token: &str) -> Result<(), String> {
    create_container(name)?;

    println!("registering runner {name} to {url}");
    docker(&[
        "run",
        "--rm",
        "--env",
        "ACTIONS_RUNNER_PRINT_LOG_TO_STDOUT=",
        "--volume",
        &format!("{name}:/home/runner"),
        IMAGE,
        "./config.sh",
        "--unattended",
        "--url",
        url,
        "--token",
        token,
        "--name",
        name,
    ])
    .map_err(|err| format!("failed to register the runner: {err}"))?;

    Ok(())
}

/// Updates every runner, or only the runner `name` if given, to the latest
/// image, returning the names of the runners updated.
///
/// The image is pulled first. Each runner whose container already runs it is
/// left untouched. Any other runner has its container stopped, letting the
/// runner end its session with GitHub, then removed, and recreated and started
/// on the same volume. A runner without a container gets a new one.
/// Each step prints a line to standard output before it runs.
///
/// # Errors
///
/// Returns an error if there's no runner named `name`, or if any `docker` call
/// fails.
#[cfg(not(tarpaulin_include))]
pub fn update(name: Option<&str>) -> Result<Vec<String>, String> {
    let mut runners = list()?;
    if let Some(name) = name {
        runners.retain(|runner| runner.name == name);
        if runners.is_empty() {
            return Err(format!("no runner named {name}"));
        }
    }

    println!("pulling image {IMAGE}");
    docker(&["pull", "--quiet", IMAGE])
        .map_err(|err| format!("failed to pull the image: {err}"))?;
    let image = docker(&["image", "inspect", "--format", "{{.Id}}", IMAGE])
        .map_err(|err| format!("failed to inspect the image: {err}"))?;

    let mut updated = Vec::new();
    for Runner { name, state, .. } in runners {
        if state != "missing" {
            let current = docker(&["container", "inspect", "--format", "{{.Image}}", &name])
                .map_err(|err| format!("failed to inspect the runner container: {err}"))?;
            if current.trim() == image.trim() {
                println!("runner {name} is up to date");
                continue;
            }

            println!("stopping container {name}");
            docker(&["stop", &name]).map_err(|err| format!("failed to stop the runner: {err}"))?;

            println!("removing container {name}");
            docker(&["rm", &name])
                .map_err(|err| format!("failed to remove the runner container: {err}"))?;
        }

        create_container(&name)?;

        println!("starting container {name}");
        docker(&["start", &name]).map_err(|err| format!("failed to start the runner: {err}"))?;
        updated.push(name);
    }
    Ok(updated)
}

/// Creates the container of the runner `name` on its volume, without starting
/// it.
#[cfg(not(tarpaulin_include))]
fn create_container(name: &str) -> Result<(), String> {
    println!("creating container {name}");
    docker(&[
        "create",
        "--name",
        name,
        "--restart",
        "unless-stopped",
        "--volume",
        &format!("{name}:/home/runner"),
        IMAGE,
        "bash",
        "-c",
        "sudo apt-get update; exec ./run.sh",
    ])
    .map_err(|err| format!("failed to create the runner container: {err}"))?;
    Ok(())
}

/// Runs `docker` with `args` and returns its standard output.
///
/// The error carries the command's output, not its arguments, so that secrets
/// passed as arguments never end up in it.
#[cfg(not(tarpaulin_include))]
fn docker(args: &[&str]) -> Result<String, String> {
    let output = Command::new("docker")
        .args(args)
        .output()
        .map_err(|err| format!("failed to run docker: {err}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "docker exited with {}\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ))
    }
}

/// Returns the repository name at the end of `url`, without a trailing `/` or
/// `.git`.
fn repo_name(url: &str) -> Result<&str, String> {
    let last = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    let name = last.strip_suffix(".git").unwrap_or(last);
    if name.is_empty() {
        return Err(format!("no repository name in URL: {url}"));
    }
    Ok(name)
}

/// Returns `<repo>-<N>` with the lowest `N`, starting from 1, that isn't in
/// `taken`.
fn next_runner_name<'a>(repo: &str, taken: impl IntoIterator<Item = &'a str>) -> String {
    let taken: HashSet<&str> = taken.into_iter().collect();
    let mut n = 1;
    loop {
        let name = format!("{repo}-{n}");
        if !taken.contains(name.as_str()) {
            return name;
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_repo_name() {
        assert_eq!(
            repo_name("https://github.com/threeal/leetspace"),
            Ok("leetspace")
        );
        assert_eq!(
            repo_name("https://github.com/threeal/leetspace/"),
            Ok("leetspace")
        );
        assert_eq!(
            repo_name("https://github.com/threeal/leetspace.git"),
            Ok("leetspace")
        );
    }

    #[test]
    fn get_repo_name_from_url_without_name() {
        assert_eq!(
            repo_name("/"),
            Err("no repository name in URL: /".to_string())
        );
    }

    #[test]
    fn parse_runners_with_states() {
        let volumes = "repo-2\thttps://github.com/owner/repo\n\
                       other-1\thttps://github.com/owner/other\n\
                       repo-1\thttps://github.com/owner/repo\n";
        let containers = "repo-1\trunning\nrepo-2\texited\nunrelated\trunning\n";
        assert_eq!(
            parse_runners(volumes, containers),
            [
                Runner {
                    name: "other-1".to_string(),
                    url: "https://github.com/owner/other".to_string(),
                    state: "missing".to_string(),
                },
                Runner {
                    name: "repo-1".to_string(),
                    url: "https://github.com/owner/repo".to_string(),
                    state: "running".to_string(),
                },
                Runner {
                    name: "repo-2".to_string(),
                    url: "https://github.com/owner/repo".to_string(),
                    state: "exited".to_string(),
                },
            ]
        );
    }

    #[test]
    fn parse_no_runners() {
        assert_eq!(parse_runners("", ""), []);
    }

    #[test]
    fn format_runners_in_columns() {
        let runners = [
            Runner {
                name: "leetspace-1".to_string(),
                url: "https://github.com/threeal/leetspace".to_string(),
                state: "running".to_string(),
            },
            Runner {
                name: "repo-10".to_string(),
                url: "https://github.com/owner/repo".to_string(),
                state: "missing".to_string(),
            },
        ];
        assert_eq!(
            format_runners(&runners),
            "leetspace-1  https://github.com/threeal/leetspace  running\n\
             repo-10      https://github.com/owner/repo         missing\n"
        );
    }

    #[test]
    fn format_no_runners() {
        assert_eq!(format_runners(&[]), "");
    }

    #[test]
    fn get_next_runner_name() {
        assert_eq!(next_runner_name("leetspace", []), "leetspace-1");
        assert_eq!(
            next_runner_name("leetspace", ["other-1", "leetspace-1", "leetspace-3"]),
            "leetspace-2"
        );
    }
}
