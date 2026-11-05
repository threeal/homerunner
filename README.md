# homerunner

A CLI for deploying and managing persistent [self-hosted GitHub Actions runners](https://docs.github.com/en/actions/hosting-your-own-runners) in [Docker](https://www.docker.com/), on Linux only. Register a runner once with a token and leave it running, keep its build cache warm across jobs, and scale to more instances with one command.

This project is at an early stage: the toolkit isn't implemented yet, and its CLI only prints a placeholder notice.

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
