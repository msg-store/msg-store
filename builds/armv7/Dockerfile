FROM rust:1.40 as builder
WORKDIR /usr/src/myapp
COPY ../../msg_store_uuid .
RUN cargo build --release