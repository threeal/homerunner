//! Functions that call `docker`, excluded from coverage in `tarpaulin.toml`.
//!
//! Only code that calls `docker` belongs here, so that everything else stays
//! covered by tests.

use std::process::Command;

use crate::{Runner, next_runner_name, parse_runners, url_name};

/// The image every runner container runs, built from `images/ubuntu-24.04`.
const IMAGE: &str = "ghcr.io/threeal/homerunner:ubuntu-24.04";

/// The label on each runner's volume holding the URL of the repository or
/// organization it's registered to, which marks the volume as a runner's.
const URL_LABEL: &str = "homerunner.url";

/// Returns every runner managed by homerunner, sorted by name.
///
/// Runners are found by the label on their volumes, so containers and volumes
/// that homerunner didn't create are left out.
///
/// # Errors
///
/// Returns an error if any `docker` call fails.
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

/// Registers a new self-hosted runner for the repository or organization at
/// `url` and starts it, returning the runner's name.
///
/// The runner is named `<name>-<N>`, after the repository or organization, using the lowest `N` not already taken by
/// a Docker volume. Its volume of the same name holds the runner's home
/// directory, and its container of the same name restarts when the Docker
/// daemon does, but not once the runner gives up, such as when its
/// registration is gone.
/// Each step prints a line to standard output before it runs.
/// If any step before starting the container fails, the container and volume
/// created so far are removed.
///
/// # Errors
///
/// Returns an error if `url` has no repository or organization name, or if
/// any `docker` call fails.
pub fn add(url: &str, token: &str) -> Result<String, String> {
    // GitHub rejects an organization URL with a trailing `/`.
    let url = url.trim_end_matches('/');
    let prefix = url_name(url)?;
    let volumes = docker(&["volume", "ls", "--format", "{{.Name}}"])
        .map_err(|err| format!("failed to list volumes: {err}"))?;
    let name = next_runner_name(prefix, volumes.lines());

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

/// Removes the runner `name` from this host.
///
/// The runner's container is stopped, letting the runner end its session with
/// GitHub, then removed, followed by its volume, which holds its registration
/// and build state. A runner without a container just has its volume removed.
/// The runner stays registered on GitHub, where it shows as offline until it's
/// removed from the runner settings of its repository or organization.
/// Each step prints a line to standard output before it runs.
///
/// # Errors
///
/// Returns an error if there's no runner named `name`, or if any `docker` call
/// fails.
pub fn remove(name: &str) -> Result<(), String> {
    let runner = list()?
        .into_iter()
        .find(|runner| runner.name == name)
        .ok_or_else(|| format!("no runner named {name}"))?;

    if runner.state != "missing" {
        println!("stopping container {name}");
        docker(&["stop", name]).map_err(|err| format!("failed to stop the runner: {err}"))?;

        println!("removing container {name}");
        docker(&["rm", name])
            .map_err(|err| format!("failed to remove the runner container: {err}"))?;
    }

    println!("removing volume {name}");
    docker(&["volume", "rm", name])
        .map_err(|err| format!("failed to remove the runner volume: {err}"))?;
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
fn create_container(name: &str) -> Result<(), String> {
    println!("creating container {name}");
    docker(&[
        "create",
        "--name",
        name,
        "--restart",
        "on-failure",
        "--volume",
        &format!("{name}:/home/runner"),
        IMAGE,
        "/opt/homerunner/run.sh",
    ])
    .map_err(|err| format!("failed to create the runner container: {err}"))?;
    Ok(())
}

/// Runs `docker` with `args` and returns its standard output.
///
/// The error carries the command's output, not its arguments, so that secrets
/// passed as arguments never end up in it.
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
