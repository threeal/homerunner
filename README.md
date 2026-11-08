# homerunner

A CLI for deploying and managing persistent [self-hosted GitHub Actions runners](https://docs.github.com/en/actions/hosting-your-own-runners) in [Docker](https://www.docker.com/), on Linux only. Register a runner once with a token and leave it running, keep its build cache warm across jobs, and scale to more instances with one command.

This project is at an early stage: for now, it can only add, list, and update runners.

## Usage

homerunner calls `docker` as the current user, so that user needs access to the Docker daemon.

To add a runner, open the repository's "New self-hosted runner" page under Settings → Actions → Runners, copy the registration token shown there, and run:

```sh
homerunner add https://github.com/<owner>/<repo> --token <TOKEN>
```

The runner is named `<repo>-<N>`, using the lowest number not already taken, and runs in a Docker container of the same name that restarts unless stopped. Its home directory, including the build state in `_work`, lives in a Docker volume of the same name. Each registration token works once, so adding another runner takes a fresh one.

The container runs homerunner's own Ubuntu 24.04 image, `ghcr.io/threeal/homerunner:ubuntu-24.04`, which ships the runner and the user it runs as, with passwordless sudo. Like GitHub-hosted Ubuntu runners, it comes with Clang, clang-format, CMake, Ninja, and pipx preinstalled. The container refreshes the apt package index whenever it starts, so workflows can run `sudo apt-get install` without `apt-get update` first, as on GitHub-hosted runners.

To list the runners homerunner manages, with the repository each is registered to and the state of its container, run:

```sh
homerunner list
```

Each push to homerunner's `main` branch publishes a new image, but a runner keeps the image its container was created from. To move every runner onto the latest image, or only the runner `<NAME>` if given, run:

```sh
homerunner update [<NAME>]
```

It pulls the image, then stops, recreates, and starts the container of each runner not on it yet, keeping its volume and registration, so it needs no token. Stopping a container cancels any job its runner is running.

## Setup

Install [rustup](https://rustup.rs/), then install the toolchain pinned in `rust-toolchain.toml`:

```sh
rustup install
```

Install [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) to run the test suite with coverage:

```sh
cargo install cargo-tarpaulin
```

Install [Lefthook](https://lefthook.dev/) and [dprint](https://dprint.dev/), then register the pre-commit hook:

```sh
lefthook install
```

## Development

Write code in `src/`. Unit tests live in a `tests` module alongside the code they cover. Run the test suite with:

```sh
cargo tarpaulin
```

The project enforces 100% code coverage on every run.

Each `git commit` runs the pre-commit hook registered during setup, which checks your changes and fixes what it can in place. If it fails, fix any reported issues, re-stage the changed files, and commit again.

After committing, push to `main` or open a pull request from another branch — CI will run the same checks across all files, plus additional checks of its own.

## Releasing

Update the version in `Cargo.toml`, push a version tag, and create a GitHub Release. To publish to [crates.io](https://crates.io/), run:

```sh
cargo publish
```
