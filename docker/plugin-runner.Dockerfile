# syntax=docker/dockerfile:1.7

FROM rust:1-slim-bookworm AS builder

WORKDIR /workspace/api

RUN apt-get update \
  && apt-get install -y --no-install-recommends build-essential ca-certificates pkg-config \
  && rm -rf /var/lib/apt/lists/*

COPY api/Cargo.toml api/Cargo.lock ./
COPY api/apps ./apps
COPY api/crates ./crates

RUN --mount=type=cache,id=1flowbase-cargo-registry,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,id=1flowbase-cargo-git,sharing=locked,target=/usr/local/cargo/git \
    --mount=type=cache,id=1flowbase-rust-target,sharing=locked,target=/workspace/api/target-cache \
    CARGO_TARGET_DIR=/workspace/api/target-cache \
      cargo build --release -p plugin-runner --bin plugin-runner \
    && cp /workspace/api/target-cache/release/plugin-runner /workspace/api/plugin-runner

FROM debian:bookworm-slim AS runtime

ARG APP_UID=1000
ARG APP_GID=1000

WORKDIR /app

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && groupadd --gid "${APP_GID}" flowbase \
  && useradd --uid "${APP_UID}" --gid "${APP_GID}" --create-home --shell /usr/sbin/nologin flowbase

COPY --from=builder /workspace/api/plugin-runner /usr/local/bin/plugin-runner

USER flowbase

EXPOSE 7801

ENTRYPOINT ["/usr/local/bin/plugin-runner"]
