FROM ubuntu:20.04 as deploy

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    clang \
    libssl1.1 ca-certificates curl llvm systemd && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN mkdir -p /etc/tycho-wallet-api && mkdir -p /var/db/tycho-wallet-api

COPY --from=builder /app/target/release/tycho-wallet-api /usr/local/bin/tycho-wallet-api
COPY --from=builder /app/scripts/contrib/config.json /etc/tycho-wallet-api/config.json

# Download external configuration file
RUN curl -so /etc/tycho-wallet-api/global-config.json \
     https://testnet.tychoprotocol.com/global-config.json

# Restart systemd-timesyncd service
# RUN systemctl enable systemd-timesyncd.service
WORKDIR /etc/tycho-wallet-api

# Default command for the container (optional)
CMD ["/usr/local/bin/tycho-wallet-api", "server", "--config", "/etc/tycho-wallet-api/config.json", "--global-config", "/etc/tycho-wallet-api/global-config.json", "--keys", "/etc/tycho-wallet-api/keys.json"]
