# syntax=docker/dockerfile:1
FROM ubuntu:22.04

RUN apt-get update && \
    apt-get upgrade && \
    apt-get install -y --no-install-recommends \
        tar wget curl clang autoconf zip unzip \
        git \
        build-essential cmake pkg-config \
        lzma-dev m4 zlib1g-dev libssl-dev ca-certificates \
        neovim

ADD --checksum=sha256:ab5a03176ee106d3f0fa90e381da478ddae405918153cca248e682cd0c4a2269 \
    https://sourceware.org/pub/bzip2/bzip2-1.0.8.tar.gz /opt/
ADD --checksum=sha256:2e8a40b01979afe8be0bbfb3de5dc1c6709fedb46d6c89c10da114ab5fc3d281 \
    https://github.com/openssl/openssl/releases/download/openssl-3.3.2/openssl-3.3.2.tar.gz /opt/
ADD --checksum=sha256:4aa0929db06c2376cc76f6d9e920dbde2ea6f1fd55bbaed6cd65167e9e39ecd3 \
    https://github.com/WebAssembly/wabt/releases/download/1.0.36/wabt-1.0.36-ubuntu-20.04.tar.gz /opt/
ADD --checksum=sha256:c6c38aab56e5de88adf6c1ebc9c3ae8da72f88ec2b656fb024eda8d4167a0bc5 \
    https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-linux.tar.gz /opt/
ADD --checksum=sha256:8bfd20c0e1d86f0402f2497cfa71c6ab62d4cd35fd704276e3140bfb71414519 \
    https://github.com/tukaani-project/xz/releases/download/v5.6.2/xz-5.6.2.tar.gz /opt/
ADD --checksum=sha256:9a93b2b7dfdac77ceba5a558a580e74667dd6fede4585b91eefb60f03b72df23 \
    https://www.zlib.net/zlib-1.3.1.tar.gz /opt/

RUN cd /opt && \
    tar -xf wasi-sdk-24.0-x86_64-linux.tar.gz && \
    rm -rf wasi-sdk-24.0-x86_64-linux.tar.gz && \
    mv wasi-sdk-24.0-x86_64-linux wasi-sdk-24.0 && \
    tar -xf openssl-3.3.2.tar.gz && \
    rm -rf openssl-3.3.2.tar.gz && \
    tar -xf zlib-1.3.1.tar.gz && \
    rm -rf  zlib-1.3.1.tar.gz && \
    tar -xf xz-5.6.2.tar.gz && \
    rm -rf  xz-5.6.2.tar.gz && \
    tar -xf bzip2-1.0.8.tar.gz && \
    rm -rf bzip2-1.0.8.tar.gz && \
    tar -xf wabt-1.0.36-ubuntu-20.04.tar.gz && \
    rm wabt-1.0.36-ubuntu-20.04.tar.gz

# faster then ADD
RUN mkdir -p /opt/cpython && cd /opt/cpython && \
    git init && \
    git remote add origin https://github.com/python/cpython.git && \
    git fetch origin e9b00cc78853373623031c657193cbe557488c0a --depth 1 && \
    git checkout e9b00cc78853373623031c657193cbe557488c0a

ENV WASI_SDK_PATH=/opt/wasi-sdk-24.0
ENV WASM32_WASI_ROOT=/opt/wasm32-wasip1-root/

RUN mkdir -p /out $WASM32_WASI_ROOT/include $WASM32_WASI_ROOT/lib $WASM32_WASI_ROOT/share

COPY ./scripts /scripts

WORKDIR /opt

RUN /scripts/build-all-deps.sh && /scripts/build-python-host.sh

COPY ./scripts-py /scripts-py
