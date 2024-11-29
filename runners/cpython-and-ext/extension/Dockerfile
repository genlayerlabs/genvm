FROM ubuntu:24.04

RUN apt-get -y update && \
    apt-get install -y --no-install-recommends \
        ruby curl \
        build-essential pkg-config \
        ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /opt/genvm/runners/cpython-and-ext/extension /opt/genvm/build-scripts

WORKDIR /opt/genvm/

COPY .genvm-monorepo-root /opt/genvm/
COPY build-scripts/install /opt/genvm/build-scripts/install
COPY runners/cpython-and-ext/extension/rust-toolchain.toml /opt/genvm/runners/cpython-and-ext/extension/rust-toolchain.toml

RUN /opt/genvm/build-scripts/install/install-deps.rb --rust-det --wasi

COPY sdk-rust/ /opt/genvm/sdk-rust/

COPY runners/cpython-and-ext/extension /opt/genvm/runners/cpython-and-ext/extension

ENV PATH="$PATH:/root/.cargo/bin"

RUN cd runners/cpython-and-ext/extension && cargo fetch --target wasm32-wasip1 --locked && cd /opt/genvm/
