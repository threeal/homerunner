//! A toolkit for deploying and managing persistent self-hosted GitHub Actions
//! runners in Docker.

use std::collections::{HashMap, HashSet};
use std::fmt::Write;

mod docker;

pub use docker::{add, list, remove, update};

/// A runner managed by homerunner.
#[derive(Debug, PartialEq, Eq)]
pub struct Runner {
    /// The name of the runner, its volume, and its container.
    pub name: String,
    /// The URL of the repository or organization the runner is registered to.
    pub url: String,
    /// The state of the runner's container, such as `running` or `exited`, or
    /// `missing` if it has no container.
    pub state: String,
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

/// Returns the name of the repository or organization at the end of `url`,
/// without a trailing `/` or `.git`.
fn url_name(url: &str) -> Result<&str, String> {
    let last = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    let name = last.strip_suffix(".git").unwrap_or(last);
    if name.is_empty() {
        return Err(format!("no repository or organization name in URL: {url}"));
    }
    Ok(name)
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
    fn get_url_name_of_repository() {
        assert_eq!(
            url_name("https://github.com/threeal/leetspace"),
            Ok("leetspace")
        );
        assert_eq!(
            url_name("https://github.com/threeal/leetspace/"),
            Ok("leetspace")
        );
        assert_eq!(
            url_name("https://github.com/threeal/leetspace.git"),
            Ok("leetspace")
        );
    }

    #[test]
    fn get_url_name_of_organization() {
        assert_eq!(url_name("https://github.com/threeal"), Ok("threeal"));
        assert_eq!(url_name("https://github.com/threeal/"), Ok("threeal"));
    }

    #[test]
    fn get_url_name_from_url_without_name() {
        assert_eq!(
            url_name("/"),
            Err("no repository or organization name in URL: /".to_string())
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
