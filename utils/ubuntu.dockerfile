FROM ubuntu:22.04

COPY ./apt-install.txt /tmp/apt.txt
RUN apt update && xargs -a /tmp/apt.txt apt install --no-install-recommends -y
RUN apt clean

RUN update-ca-certificates

ENV DEBIAN_FRONTEND=noninteractive
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | env \
    RUSTUP_HOME=/nfs/rust/rustup CARGO_HOME=/nfs/rust/cargo \
    sh -s -- --default-toolchain none --profile minimal --no-modify-path -y
ENV RUSTUP_HOME=/nfs/rust/rustup
ENV PATH="${PATH}:/nfs/rust/cargo/bin"
RUN RUSTUP_HOME=/nfs/rust/rustup CARGO_HOME=/nfs/rust/cargo \
    rustup install nightly

WORKDIR /home/devContainer