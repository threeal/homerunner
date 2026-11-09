# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## About This Repository

homerunner is a Rust library and CLI for deploying and managing persistent self-hosted GitHub Actions runners in Docker, on Linux only. For now, it has four commands: `homerunner add` registers a runner and starts it in a container backed by its own volume, `homerunner list` lists the runners it manages, found by a label on their volumes, `homerunner update` recreates their containers on the latest image, and `homerunner remove` deletes a runner's container and volume.

Runners run the images under `images/`, one directory per image, each listed in `docker-bake.hcl`. CI builds them with `docker buildx bake`, and `cd.yaml` publishes each to `ghcr.io/threeal/homerunner:<directory>` on every push to `main`. A runner's volume is seeded from the image's `/home/runner` only once, so install nothing there but the runner itself. Tools in an image should behave as on the matching GitHub-hosted runner image, including their versions and where they install. Images build reproducibly, so a push that changes no input publishes the same digest and `homerunner update` leaves runners alone: provenance is disabled, `docker-bake.hcl` fixes `SOURCE_DATE_EPOCH`, `cd.yaml` rewrites timestamps to it, and each step removes anything that records when it ran.

Code that only calls `docker` is marked `#[cfg(not(tarpaulin_include))]` to exclude it from coverage; don't mock Docker to test it.

## Gotchas

- `lefthook run pre-commit` skips every job when nothing is staged, even ones that ignore the staged file list — pass `--all-files` to run it outside an actual commit.
- The pre-commit hook fixes files in place, and the run fails if any file changes. Report that failure and leave the fixes for the user to review and re-stage before committing again.
- The test command is `cargo tarpaulin`, not `cargo test` — only it applies the 100% coverage threshold from `tarpaulin.toml`.
- `.gitignore` ignores every dotfile and dotdir (`.*`) — a new one needs an explicit `!` exception there or it silently won't be tracked.
