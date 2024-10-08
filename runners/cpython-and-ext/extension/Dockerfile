FROM ubuntu:22.04

RUN apt-get update && \
    apt-get upgrade && \
    apt-get install -y --no-install-recommends \
        ruby curl \
        build-essential pkg-config \
        ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /opt/genvm/runners/cpython-and-ext/extension/

WORKDIR /opt/genvm/

COPY .genvm-monorepo-root /opt/genvm/
COPY build-scripts/ /opt/genvm/build-scripts/
COPY runners/cpython-and-ext/extension/rust-toolchain.toml /opt/genvm/runners/cpython-and-ext/extension/

RUN /opt/genvm/build-scripts/install-deps.rb --rust-det

COPY runners/cpython-and-ext/extension/ /opt/genvm/runners/cpython-and-ext/extension/
COPY sdk-rust/ /opt/genvm/sdk-rust/
