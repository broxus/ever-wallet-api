#!/bin/bash

set -eux

echo "INFO: copy binaries..."

wget -q https://storage.googleapis.com/broxus-infrastructure/sqlx -O /usr/local/bin/sqlx
chmod +x /usr/local/bin/sqlx

cd /tmp/build/ton-wallet-api
curl -sSO https://dl.google.com/cloudagents/add-google-cloud-ops-agent-repo.sh
sudo bash add-google-cloud-ops-agent-repo.sh --also-install

mkdir -p /opt/ton/ton-wallet-api
mkdir -p /var/ton/ton-wallet-api

cp -r /tmp/build/ton-wallet-api/scripts/* /opt/ton/ton-wallet-api
chmod +x /opt/ton/ton-wallet-api/*

rm -rf /tmp/build/ton-wallet-api

echo "INFO: create user..."

addgroup ton
adduser --system --disabled-login --shell /bin/false --home=/opt/ton ton
adduser ton ton
chown ton:ton -R /opt/ton

echo "INFO: create service..."

cat > "/etc/systemd/system/ton-wallet-api.service" << EOL
[Unit]
Description=ton-wallet-api
After=network.target
StartLimitIntervalSec=0

[Service]
User=ton
Group=ton
Type=simple
Restart=always
RestartSec=60
WorkingDirectory=/opt/ton/ton-wallet-api
ExecStart=bash ./run.sh

[Install]
WantedBy=multi-user.target
EOL

systemctl enable ton-wallet-api

cat > "/etc/systemd/system/ton-wallet-api-updater.service" << EOL
[Unit]
Description=ton-wallet-api-updater
After=network.target
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=60
WorkingDirectory=/opt/ton/ton-wallet-api
ExecStart=bash ./updater.sh

[Install]
WantedBy=multi-user.target
EOL

systemctl enable ton-wallet-api-updater

cat > "/etc/systemd/system/ton-wallet-api-create-fs.service" << EOL
[Unit]
Description=ton-wallet-api-create-fs
Requires=local-fs.target
After=local-fs.target
StartLimitIntervalSec=0

[Service]
Type=oneshot
WorkingDirectory=/opt/ton/ton-wallet-api
ExecStart=bash ./create-fs.sh

[Install]
WantedBy=multi-user.target
EOL

systemctl enable ton-wallet-api-create-fs
