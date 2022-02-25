FROM ubuntu:latest as builder
WORKDIR /usr/src/myapp
# COPY . .
RUN echo 'debconf debconf/frontend select Noninteractive' | debconf-set-selections && \
    apt update && apt upgrade -y && \
    apt install -y curl \
        dialog \
        apt-utils \
        build-essential \
        libleveldb-dev \
        libsnappy-dev \
        cmake && \
    curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh -s -- -y && \
    # ~/.cargo/bin/rustup target add armv7-unknown-linux-gnueabihf && \
    ~/.cargo/bin/cargo install msg-store-http-server --version 0.1.4

FROM debian:buster-slim
ENV NODE_ID=0
EXPOSE 8080
VOLUME /msg-store
RUN apt-get update && apt-get upgrade -y && rm -rf /var/lib/apt/lists/*
COPY --from=builder /root/.cargo/bin/msg-store-http-server /usr/local/bin/msg-store-http-server
CMD msg-store-http-server --host=0.0.0.0 --node-id=$NODE_ID --database=leveldb --leveldb-path=/msg-store/leveldb --file-storage --file-storage-path=/msg-store/file-storage
