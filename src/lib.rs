//! A toolkit for deploying and managing persistent self-hosted GitHub Actions
//! runners in Docker.

use std::collections::HashSet;
use std::process::Command;

/// The image every runner container runs, which already ships the runner.
const IMAGE: &str = "ghcr.io/actions/actions-runner:latest";

/// Registers a new self-hosted runner for the repository at `url` and starts
/// it, returning the runner's name.
///
/// The runner is named `<repo>-<N>`, using the lowest `N` not already taken by
/// a Docker volume. Its volume of the same name holds the runner's home
/// directory, and its container of the same name restarts unless stopped.
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

    docker(&[
        "volume",
        "create",
        "--label",
        &format!("homerunner.url={url}"),
        &name,
    ])
    .map_err(|err| format!("failed to create the runner volume: {err}"))?;

    if let Err(err) = create_and_register(&name, url, token) {
        let _ = docker(&["rm", "--force", &name]);
        let _ = docker(&["volume", "rm", &name]);
        return Err(err);
    }

    docker(&["start", &name]).map_err(|err| format!("failed to start the runner: {err}"))?;
    Ok(name)
}

/// Creates the container of the runner `name` without starting it, then
/// registers the runner with GitHub in a throwaway container.
#[cfg(not(tarpaulin_include))]
fn create_and_register(name: &str, url: &str, token: &str) -> Result<(), String> {
    let volume = format!("{name}:/home/runner");

    docker(&[
        "create",
        "--name",
        name,
        "--restart",
        "unless-stopped",
        "--volume",
        &volume,
        IMAGE,
        "./run.sh",
    ])
    .map_err(|err| format!("failed to create the runner container: {err}"))?;

    docker(&[
        "run",
        "--rm",
        "--env",
        "ACTIONS_RUNNER_PRINT_LOG_TO_STDOUT=",
        "--volume",
        &volume,
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
    fn get_next_runner_name() {
        assert_eq!(next_runner_name("leetspace", []), "leetspace-1");
        assert_eq!(
            next_runner_name("leetspace", ["other-1", "leetspace-1", "leetspace-3"]),
            "leetspace-2"
        );
    }
}
