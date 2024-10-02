FROM ubuntu:22.04

RUN apt-get update && \
    apt-get upgrade && \
    apt-get install -y --no-install-recommends \
        ruby \
        tar wget curl clang autoconf zip unzip \
        build-essential pkg-config \
        ca-certificates && \
    mkdir -p /opt/genvm/runners/cpython-and-ext/extension/


COPY runners/cpython-and-ext/extension/ /opt/genvm/runners/cpython-and-ext/extension/
COPY sdk-rust/ /opt/genvm/sdk-rust/
COPY rust-toolchain.toml .genvm-monorepo-root /opt/genvm/
COPY build-scripts/ /opt/genvm/build-scripts/

WORKDIR /opt/genvm/

RUN ls

RUN /opt/genvm/build-scripts/install-deps.rb --rust
