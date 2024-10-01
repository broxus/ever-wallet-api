FROM ubuntu:20.04 as deploy

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    clang \
    libssl1.1 ca-certificates curl llvm systemd && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN mkdir -p /etc/ton-wallet-api && mkdir -p /var/db/ton-wallet-api

COPY --from=builder /app/target/release/ton-wallet-api /usr/local/bin/ton-wallet-api
COPY --from=builder /app/scripts/contrib/config.yaml /etc/ton-wallet-api/config.yaml

# Download external configuration file
RUN curl -so /etc/ton-wallet-api/ton-global.config.json \
    https://raw.githubusercontent.com/tonlabs/main.ton.dev/master/configs/ton-global.config.json

# Restart systemd-timesyncd service
RUN systemctl enable systemd-timesyncd.service
WORKDIR /etc/ton-wallet-api

# Default command for the container (optional)
CMD ["/usr/local/bin/ton-wallet-api", "server", "--config", "/etc/ton-wallet-api/config.yaml", "--global-config", "/etc/ton-wallet-api/ton-global.config.json"]
