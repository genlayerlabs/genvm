FROM ubuntu:22.04

RUN apt-get update && \
    apt-get upgrade && \
    apt-get install -y --no-install-recommends \
        tar wget curl clang autoconf zip unzip \
        git \
        build-essential cmake pkg-config \
        lzma-dev m4 zlib1g-dev libssl-dev ca-certificates \
        neovim

RUN apt-get install -y --no-install-recommends


RUN cd /opt && \
    wget -q https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-24/wasi-sdk-24.0-x86_64-linux.tar.gz && \
    tar -xf wasi-sdk-24.0-x86_64-linux.tar.gz && \
    rm -rf wasi-sdk-24.0-x86_64-linux.tar.gz && \
    mv wasi-sdk-24.0-x86_64-linux wasi-sdk-24.0 && \
    wget -q https://github.com/openssl/openssl/releases/download/openssl-3.3.2/openssl-3.3.2.tar.gz && \
    tar -xf openssl-3.3.2.tar.gz && \
    rm -rf openssl-3.3.2.tar.gz && \
    wget -q https://www.zlib.net/zlib-1.3.1.tar.gz && \
    tar -xf zlib-1.3.1.tar.gz && \
    rm -rf  zlib-1.3.1.tar.gz && \
    wget -q https://github.com/tukaani-project/xz/releases/download/v5.6.2/xz-5.6.2.tar.gz && \
    tar -xf xz-5.6.2.tar.gz && \
    rm -rf  xz-5.6.2.tar.gz && \
    wget -q https://sourceware.org/pub/bzip2/bzip2-1.0.8.tar.gz && \
    tar -xf bzip2-1.0.8.tar.gz && \
    rm -rf bzip2-1.0.8.tar.gz

RUN mkdir -p /opt/cpython && cd /opt/cpython && \
    git init && \
    git remote add origin https://github.com/python/cpython.git && \
    git fetch origin e9b00cc78853373623031c657193cbe557488c0a --depth 1 && \
    git checkout e9b00cc78853373623031c657193cbe557488c0a

ENV WASI_SDK_PATH=/opt/wasi-sdk-24.0
ENV WASM32_WASI_ROOT=/opt/wasm32-wasip1-root/

RUN mkdir -p /out $WASM32_WASI_ROOT/include $WASM32_WASI_ROOT/lib $WASM32_WASI_ROOT/share

COPY ./scripts /scripts
