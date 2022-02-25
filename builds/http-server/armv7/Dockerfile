FROM ubuntu:latest as builder
WORKDIR /usr/src/myapp
ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=/usr/bin/arm-linux-gnueabihf-gcc
RUN echo 'debconf debconf/frontend select Noninteractive' | debconf-set-selections && \
    apt update && apt upgrade -y && \
    apt install -y curl \
        build-essential \
        libleveldb-dev \
        libsnappy-dev \
        dialog \
        apt-utils \
        cmake \
        libc6-armel-cross \
        libc6-dev-armel-cross \
        binutils-arm-linux-gnueabi \
        libncurses5-dev \
        bison \
        flex \
        libssl-dev \
        bc \
        g++-arm-linux-gnueabihf \
        gcc-arm-linux-gnueabihf && \
    curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh -s -- -y && \
    ~/.cargo/bin/rustup target add armv7-unknown-linux-gnueabihf && \
    ~/.cargo/bin/cargo install msg-store-http-server --version 0.1.4 --target armv7-unknown-linux-gnueabihf

FROM debian:buster-slim
ARG NODE_ID=0
EXPOSE 8080
VOLUME /msg-store
RUN apt-get update && apt-get upgrade -y && rm -rf /var/lib/apt/lists/*
COPY --from=builder /root/.cargo/bin/msg-store-http-server /usr/local/bin/msg-store-http-server
CMD msg-store-http-server --node-id=$NODE_ID --database=leveldb --leveldb-path=/msg-store/leveldb --file-storage --file-storage-path=/msg-store/file-storage
