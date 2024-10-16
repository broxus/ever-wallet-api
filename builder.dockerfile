# Use the official Ubuntu 20.04 as a base image
FROM ubuntu:20.04 AS base

# Update the package repository and install dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends --reinstall ca-certificates \
    build-essential \
    llvm \
    clang \
    curl \
    libssl-dev \
    pkg-config \
    systemd \
    && rm -rf /var/lib/apt/lists/*

# Install Rust using rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain 1.74.1 -y

# Add Rust to the environment for future use
ENV PATH="/root/.cargo/bin:${PATH}"

# Install sqlx-cli using cargo
RUN cargo install sqlx-cli
# Set up the database

FROM base AS builder
# Set working directory to /app (change if necessary)
WORKDIR /app

# Copy the source code into the Docker image
COPY . /app

# Define a build argument to pass in the network (default to "Everscale")
ARG NETWORK="everscale"
ARG DATABASE_URL=postgres://everscale:everscale@localhost:5432/everscale

# Migrations first, otherwise it may not compile
RUN cargo sqlx database create --database-url "$DATABASE_URL"
RUN cargo sqlx migrate run --database-url "$DATABASE_URL"

# Build the project based on the network variable
RUN if [ "$NETWORK" = "everscale" ]; then \
      RUSTFLAGS="-C target_cpu=native" SQLX_OFFLINE=true cargo build --release; \
    elif [ "$NETWORK" = "venom" ]; then \
      RUSTFLAGS="-C target_cpu=native" SQLX_OFFLINE=true cargo build --release --features venom; \
    else \
      echo 'ERROR: Unexpected network'; \
      exit 1; \
    fi
