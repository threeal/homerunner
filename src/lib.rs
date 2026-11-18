//! A toolkit for deploying and managing persistent self-hosted GitHub Actions
//! runners in Docker.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

mod docker;

pub use docker::{add, list, remove, update};

/// A runner managed by homerunner.
#[derive(Debug, PartialEq, Eq)]
pub struct Runner {
    /// The name of the runner on GitHub and in every command.
    pub name: String,
    /// The name of the runner's volume and container, `homerunner-<name>`, or
    /// `<name>` for a runner added before homerunner prefixed it.
    pub docker_name: String,
    /// The URL of the repository or organization the runner is registered to.
    pub url: String,
    /// The state of the runner's container, such as `running` or `exited`, or
    /// `missing` if it has no container.
    pub state: String,
}

/// Returns the runners in `volumes`, lines of a volume's name, name label, and
/// URL label, each with the state of the container of the same name in
/// `containers`, lines of a container's name and state, sorted by name.
///
/// A volume without a name label belongs to a runner added before homerunner
/// prefixed Docker names, so the runner is named after the volume.
fn parse_runners(volumes: &str, containers: &str) -> Vec<Runner> {
    let states: HashMap<&str, &str> = containers
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .collect();
    let mut runners: Vec<Runner> = volumes
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            Some((fields.next()?, fields.next()?, fields.next()?))
        })
        .map(|(docker_name, name, url)| Runner {
            name: if name.is_empty() { docker_name } else { name }.to_string(),
            docker_name: docker_name.to_string(),
            url: url.to_string(),
            state: states.get(docker_name).unwrap_or(&"missing").to_string(),
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

/// Returns `hostname` as a prefix for runner names, which must also be valid
/// Docker volume and container names: the part before the first `.`,
/// lowercased, with every character other than a letter, digit, or `-`
/// replaced by `-`, and without leading or trailing `-`.
fn host_prefix(hostname: &str) -> Result<String, String> {
    let host = hostname.trim().split('.').next().unwrap_or_default();
    let prefix: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let prefix = prefix.trim_matches('-');
    if prefix.is_empty() {
        return Err(format!("no valid runner name in hostname: {hostname}"));
    }
    Ok(prefix.to_string())
}

/// Checks that `prefix`, given for runner names, is also valid in Docker
/// volume and container names: holding only lowercase letters, digits, and
/// `-`, and not starting or ending with `-`.
fn check_prefix(prefix: &str) -> Result<(), String> {
    let valid = prefix
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !valid || prefix.is_empty() || prefix.starts_with('-') || prefix.ends_with('-') {
        return Err(format!(
            "invalid runner name prefix: {prefix:?}, which must hold only \
             lowercase letters, digits, and `-`, and not start or end with `-`"
        ));
    }
    Ok(())
}

/// Returns `<prefix>-<N>` with the lowest `N`, starting from 1, that isn't in
/// `taken`.
fn next_runner_name<'a>(prefix: &str, taken: impl IntoIterator<Item = &'a str>) -> String {
    let taken: HashSet<&str> = taken.into_iter().collect();
    let mut n = 1;
    loop {
        let name = format!("{prefix}-{n}");
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
    fn get_host_prefix() {
        assert_eq!(host_prefix("devbox\n"), Ok("devbox".to_string()));
        assert_eq!(host_prefix("dev-box-2"), Ok("dev-box-2".to_string()));
    }

    #[test]
    fn get_host_prefix_of_unusual_hostname() {
        assert_eq!(host_prefix("DevBox.example.com"), Ok("devbox".to_string()));
        assert_eq!(host_prefix("_dev_box_"), Ok("dev-box".to_string()));
    }

    #[test]
    fn get_host_prefix_without_valid_name() {
        assert_eq!(
            host_prefix(".local"),
            Err("no valid runner name in hostname: .local".to_string())
        );
    }

    #[test]
    fn check_valid_prefix() {
        assert_eq!(check_prefix("build"), Ok(()));
        assert_eq!(check_prefix("ci-box-2"), Ok(()));
    }

    #[test]
    fn check_invalid_prefix() {
        for prefix in ["", "Build", "ci_box", "ci.box", "-ci", "ci-"] {
            assert_eq!(
                check_prefix(prefix),
                Err(format!(
                    "invalid runner name prefix: {prefix:?}, which must hold only \
                     lowercase letters, digits, and `-`, and not start or end with `-`"
                ))
            );
        }
    }

    #[test]
    fn parse_runners_with_states() {
        let volumes = "homerunner-host-2\thost-2\thttps://github.com/owner/repo\n\
                       homerunner-host-3\thost-3\thttps://github.com/owner/other\n\
                       host-1\t\thttps://github.com/owner/repo\n";
        let containers = "host-1\trunning\nhomerunner-host-2\texited\nunrelated\trunning\n";
        assert_eq!(
            parse_runners(volumes, containers),
            [
                Runner {
                    name: "host-1".to_string(),
                    docker_name: "host-1".to_string(),
                    url: "https://github.com/owner/repo".to_string(),
                    state: "running".to_string(),
                },
                Runner {
                    name: "host-2".to_string(),
                    docker_name: "homerunner-host-2".to_string(),
                    url: "https://github.com/owner/repo".to_string(),
                    state: "exited".to_string(),
                },
                Runner {
                    name: "host-3".to_string(),
                    docker_name: "homerunner-host-3".to_string(),
                    url: "https://github.com/owner/other".to_string(),
                    state: "missing".to_string(),
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
                docker_name: "leetspace-1".to_string(),
                url: "https://github.com/threeal/leetspace".to_string(),
                state: "running".to_string(),
            },
            Runner {
                name: "repo-10".to_string(),
                docker_name: "homerunner-repo-10".to_string(),
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
        assert_eq!(next_runner_name("devbox", []), "devbox-1");
        assert_eq!(
            next_runner_name("devbox", ["leetspace-1", "devbox-1", "devbox-3"]),
            "devbox-2"
        );
    }
}
