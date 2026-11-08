FROM scratch AS cmake

ARG CMAKE_VERSION=3.31.6

ADD "https://github.com/Kitware/CMake/releases/download/v${CMAKE_VERSION}/cmake-${CMAKE_VERSION}-linux-x86_64.tar.gz" /cmake.tar.gz

FROM scratch AS ninja

ARG NINJA_VERSION=1.13.2

ADD "https://github.com/ninja-build/ninja/releases/download/v${NINJA_VERSION}/ninja-linux.zip" /ninja.zip

FROM ubuntu:24.04

LABEL org.opencontainers.image.description="Ubuntu 24.04 image for self-hosted GitHub Actions runners deployed by homerunner"
LABEL org.opencontainers.image.source=https://github.com/threeal/homerunner

ARG RUNNER_VERSION=2.338.2

ENV DEBIAN_FRONTEND=noninteractive
ENV RUNNER_MANUALLY_TRAP_SIG=1
ENV ACTIONS_RUNNER_PRINT_LOG_TO_STDOUT=1
ENV ImageOS=ubuntu24

# As on GitHub-hosted runners, pipx installs apps outside the home directory,
# with their commands on the PATH.
ENV PIPX_HOME=/opt/pipx
ENV PIPX_BIN_DIR=/opt/pipx_bin
ENV PATH=/opt/pipx_bin:$PATH

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
    && install -d -o runner -g runner "$PIPX_HOME" "$PIPX_BIN_DIR"

WORKDIR /home/runner

# Only the runner goes in the home directory, since a runner's volume is
# seeded from it once and never picks up later changes to the image.
RUN curl -fsSL "https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz" | tar -xz \
    && ./bin/installdependencies.sh \
    && rm -rf /var/lib/apt/lists/* \
    && chown -R runner:runner /home/runner

USER runner
