FROM ubuntu:latest as builder
WORKDIR /usr/src/myapp
RUN apt update && apt upgrade -y
RUN apt install -y curl
RUN curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh -s -- -y
RUN apt install -y build-essential
run apt install -y libleveldb-dev libsnappy-dev
RUN apt install -y dialog apt-utils
# run export DEBIAN_FRONTEND=noninteractive
RUN echo 'debconf debconf/frontend select Noninteractive' | debconf-set-selections
RUN apt install -y -q cmake
RUN apt install -y libc6-armel-cross libc6-dev-armel-cross binutils-arm-linux-gnueabi libncurses5-dev bison flex libssl-dev bc
RUN ~/.cargo/bin/rustup target add armv7-unknown-linux-gnueabihf
RUN apt install -y gcc-arm-linux-gnueabihf
RUN apt install -y g++-arm-linux-gnueabihf
# RUN sh -c "./rustup-install -y && source $HOME/.cargo/env && "
# RUN export PATH=$PATH:$HOME/.cargo/env && cargo build --release
# RUN source $HOME/.cargo/env
# RUN ls -la ~/.cargo/bin
COPY . .
RUN ~/.cargo/bin/cargo fetch
# ENV CC=arm-linux-gnueabihf-gcc
# ENV CXX=arm-linux-gnueabihf-g++
# RUN ls /usr/bin

ENV CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER=/usr/bin/arm-linux-gnueabihf-gcc

RUN ~/.cargo/bin/cargo build --release --target armv7-unknown-linux-gnueabihf
