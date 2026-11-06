FROM ubuntu:24.04

LABEL org.opencontainers.image.description="Ubuntu 24.04 image for self-hosted GitHub Actions runners deployed by homerunner"
LABEL org.opencontainers.image.source=https://github.com/threeal/homerunner

ARG RUNNER_VERSION=2.338.2

ENV DEBIAN_FRONTEND=noninteractive
ENV RUNNER_MANUALLY_TRAP_SIG=1
ENV ACTIONS_RUNNER_PRINT_LOG_TO_STDOUT=1
ENV ImageOS=ubuntu24

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git sudo \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --uid 1001 runner \
    && printf '%s\n' 'runner ALL=(ALL) NOPASSWD:ALL' 'Defaults env_keep += "DEBIAN_FRONTEND"' > /etc/sudoers.d/runner \
    && chmod 0440 /etc/sudoers.d/runner

WORKDIR /home/runner

# Only the runner goes in the home directory, since a runner's volume is
# seeded from it once and never picks up later changes to the image.
RUN curl -fsSL "https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-x64-${RUNNER_VERSION}.tar.gz" | tar -xz \
    && ./bin/installdependencies.sh \
    && rm -rf /var/lib/apt/lists/* \
    && chown -R runner:runner /home/runner

USER runner
