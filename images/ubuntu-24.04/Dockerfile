FROM scratch AS cmake

ARG CMAKE_VERSION=3.31.6

ADD "https://github.com/Kitware/CMake/releases/download/v${CMAKE_VERSION}/cmake-${CMAKE_VERSION}-linux-x86_64.tar.gz" /cmake.tar.gz

FROM scratch AS ninja

ARG NINJA_VERSION=1.13.2

ADD "https://github.com/ninja-build/ninja/releases/download/v${NINJA_VERSION}/ninja-linux.zip" /ninja.zip

FROM scratch AS rustup

ARG RUSTUP_VERSION=1.29.3

ADD --chmod=755 "https://static.rust-lang.org/rustup/archive/${RUSTUP_VERSION}/x86_64-unknown-linux-gnu/rustup-init" /rustup-init

FROM ubuntu:24.04

LABEL org.opencontainers.image.description="Ubuntu 24.04 image for self-hosted GitHub Actions runners deployed by homerunner"
LABEL org.opencontainers.image.source=https://github.com/threeal/homerunner

ARG RUNNER_VERSION=2.338.2
ARG RUST_VERSION=1.99.1

ENV DEBIAN_FRONTEND=noninteractive
ENV RUNNER_MANUALLY_TRAP_SIG=1
ENV ACTIONS_RUNNER_PRINT_LOG_TO_STDOUT=1
ENV ImageOS=ubuntu24
ENV ACTIONS_RUNNER_HOOK_JOB_STARTED=/opt/homerunner/job-started.sh

# As on GitHub-hosted runners, pipx installs apps outside the home directory,
# with their commands on the PATH.
ENV PIPX_HOME=/opt/pipx
ENV PIPX_BIN_DIR=/opt/pipx_bin
ENV PATH=/opt/pipx_bin:$PATH

# Unlike on GitHub-hosted runners, rustup and Cargo live outside the home
# directory, which only ever holds the runner, with their commands on the PATH.
ENV RUSTUP_HOME=/opt/rustup
ENV CARGO_HOME=/opt/cargo
ENV PATH=/opt/cargo/bin:$PATH

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git sudo \
    && rm -rf /var/lib/apt/lists/*

# Tools preinstalled on GitHub-hosted Ubuntu runners, each installed from the
# same source and at the same version as there.
RUN apt-get update \
    && apt-get install -y --no-install-recommends clang clang-format python3 python3-pip python3-venv unzip \
    && rm -rf /var/lib/apt/lists/*

# pipx is pinned to the last version that works with the packaging library
# Ubuntu 24.04 ships, and pip may install into the system Python.
RUN printf '%s\n' '[global]' 'break-system-packages = true' > /etc/pip.conf \
    && python3 -m pip install --no-cache-dir pipx==1.16.7

RUN --mount=from=cmake,target=/mnt/cmake tar -xzf /mnt/cmake/cmake.tar.gz -C /usr/local --strip-components=1

RUN --mount=from=ninja,target=/mnt/ninja unzip -q /mnt/ninja/ninja.zip -d /usr/local/bin

RUN useradd --create-home --uid 1001 runner \
    && printf '%s\n' 'runner ALL=(ALL) NOPASSWD:ALL' 'Defaults env_keep += "DEBIAN_FRONTEND"' > /etc/sudoers.d/runner \
    && chmod 0440 /etc/sudoers.d/runner \
    && install -d -o runner -g runner "$PIPX_HOME" "$PIPX_BIN_DIR" "$RUSTUP_HOME" "$CARGO_HOME"

# Rust is installed as the runner user, so jobs can add toolchains and
# components without sudo.
USER runner

RUN --mount=from=rustup,target=/mnt/rustup /mnt/rustup/rustup-init -y --no-modify-path --profile=minimal --default-toolchain="${RUST_VERSION}" --component=rustfmt,clippy \
    && rm -rf "${CARGO_HOME}/registry"

USER root

WORKDIR /home/runner

# Only the runner goes in the home directory, since a runner's volume is
# seeded from it once and never picks up later changes to the image.
RUN curl -fsSL "https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz" | tar -xz \
    && ./bin/installdependencies.sh \
    && rm -rf /var/lib/apt/lists/* \
    && chown -R runner:runner /home/runner

# Before each job, the runner refreshes the apt package index if it hasn't in
# the last hour, so workflows can install packages without updating it first.
COPY --chmod=755 job-started.sh /opt/homerunner/job-started.sh

USER runner
