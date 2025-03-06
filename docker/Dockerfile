# Steps on Docker build
# 1. Use `cargo chef` from linux/amd64 and linux/arm64 as base libs
# 2. Use another `cargo chef` as a base image
# 3. Use base image as planner then builder
# 4. Copy runtime dependencies from different chef images in step 1
# 5. Install system dependencies on builder for cross-compilation
# 6. Build dependecies for each architecture

ARG CARGO_CHEF_TAG=latest-rust-1.83.0

FROM --platform=linux/amd64 lukemathwalker/cargo-chef:$CARGO_CHEF_TAG AS amd64-chef
FROM --platform=linux/arm64 lukemathwalker/cargo-chef:$CARGO_CHEF_TAG AS arm64-chef

# Base image for build stage
FROM --platform=$BUILDPLATFORM lukemathwalker/cargo-chef:$CARGO_CHEF_TAG AS chef
WORKDIR /app

# Planner
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

ARG TARGETPLATFORM
ARG TARGETARCH

# Depedent on each architecture
WORKDIR /build/linux/arm64/lib/aarch64-linux-gnu
WORKDIR /build/linux/amd64/lib/x86_64-linux-gnu
WORKDIR /build/linux/amd64/lib64

# Independent
WORKDIR /build/${TARGETPLATFORM}/etc/ssl/certs
WORKDIR /build/${TARGETPLATFORM}/app

# Runtime deps for ARM64
COPY --from=arm64-chef \
    /lib/aarch64-linux-gnu/libssl.so.3 \
    /lib/aarch64-linux-gnu/libcrypto.so.3 \
    /lib/aarch64-linux-gnu/libgcc_s.so.1 \
    /lib/aarch64-linux-gnu/libm.so.6 \
    /lib/aarch64-linux-gnu/libc.so.6 \
    /build/linux/arm64/lib/aarch64-linux-gnu/

COPY --from=arm64-chef \
    /lib/ld-linux-aarch64.so.1 \
    /build/linux/arm64/lib/

# Runtime deps for AMD64
COPY --from=amd64-chef \
    /lib/x86_64-linux-gnu/libssl.so.3 \
    /lib/x86_64-linux-gnu/libcrypto.so.3 \
    /lib/x86_64-linux-gnu/libgcc_s.so.1 \
    /lib/x86_64-linux-gnu/libm.so.6 \
    /lib/x86_64-linux-gnu/libc.so.6 \
    /build/linux/amd64/lib/x86_64-linux-gnu/

COPY --from=amd64-chef \
    /lib64/ld-linux-x86-64.so.2 \
    /build/linux/amd64/lib64/

# Common files
COPY --from=arm64-chef \
    /etc/ssl/certs/ca-certificates.crt \
    /build/linux/arm64/etc/ssl/certs/

COPY --from=amd64-chef \
    /etc/ssl/certs/ca-certificates.crt \
    /build/linux/amd64/etc/ssl/certs/

WORKDIR /app

# Install deps
RUN dpkg --add-architecture arm64 \
    && apt-get update \
    && apt-get install -y \
    protobuf-compiler \
    g++-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    libssl-dev:arm64 \
    ca-certificates \
    && rustup target add aarch64-unknown-linux-gnu \
    && rustup toolchain install stable-aarch64-unknown-linux-gnu \
    && rm -rf /var/lib/apt/lists/*

COPY . /app

# Build deps
RUN case ${TARGETARCH} in \
        arm64) PKG_CONFIG_SYSROOT_DIR=/ CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc cargo chef cook --target=aarch64-unknown-linux-gnu --release --recipe-path recipe.json ;; \
        amd64) cargo build --release ;; \
        *) exit 1 ;; \
    esac

# Copy deps
RUN set -ex; \
    case ${TARGETARCH} in \
        arm64) target='/app/target/aarch64-unknown-linux-gnu/release' ;; \
        amd64) target='/app/target/release' ;; \
        *) exit 1 ;; \
    esac; \
    cp $target/db-migrate /build/${TARGETPLATFORM}/app && \
    cp $target/server /build/${TARGETPLATFORM}/app

FROM alpine:latest AS runtime

ARG TARGETPLATFORM
ARG TARGETARCH

WORKDIR /app

COPY --from=builder /build/${TARGETPLATFORM} /

EXPOSE 50051

CMD ["/bin/sh", "-c", "./db-migrate && ./server"]