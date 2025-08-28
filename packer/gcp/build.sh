#!/bin/bash

set -eux

echo "INFO: copy binaries..."

wget -q https://storage.googleapis.com/broxus-infrastructure/sqlx -O /usr/local/bin/sqlx
chmod +x /usr/local/bin/sqlx

cd /tmp/build/tycho-wallet-api
curl -sSO https://dl.google.com/cloudagents/add-google-cloud-ops-agent-repo.sh
sudo bash add-google-cloud-ops-agent-repo.sh --also-install

mkdir -p /opt/ton/tycho-wallet-api
mkdir -p /var/ton/tycho-wallet-api

cp -r /tmp/build/tycho-wallet-api/scripts/* /opt/ton/tycho-wallet-api
chmod +x /opt/ton/tycho-wallet-api/*

rm -rf /tmp/build/tycho-wallet-api

echo "INFO: create user..."

addgroup ton
adduser --system --disabled-login --shell /bin/false --home=/opt/ton ton
adduser ton ton
chown ton:ton -R /opt/ton

echo "INFO: create service..."

cat > "/etc/systemd/system/tycho-wallet-api.service" << EOL
[Unit]
Description=tycho-wallet-api
After=network.target
StartLimitIntervalSec=0

[Service]
User=ton
Group=ton
Type=simple
Restart=always
RestartSec=60
WorkingDirectory=/opt/ton/tycho-wallet-api
ExecStart=bash ./run.sh

[Install]
WantedBy=multi-user.target
EOL

systemctl enable tycho-wallet-api

cat > "/etc/systemd/system/tycho-wallet-api-updater.service" << EOL
[Unit]
Description=tycho-wallet-api-updater
After=network.target
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=60
WorkingDirectory=/opt/ton/tycho-wallet-api
ExecStart=bash ./updater.sh

[Install]
WantedBy=multi-user.target
EOL

systemctl enable tycho-wallet-api-updater

cat > "/etc/systemd/system/tycho-wallet-api-create-fs.service" << EOL
[Unit]
Description=tycho-wallet-api-create-fs
Requires=local-fs.target
After=local-fs.target
StartLimitIntervalSec=0

[Service]
Type=oneshot
WorkingDirectory=/opt/ton/tycho-wallet-api
ExecStart=bash ./create-fs.sh

[Install]
WantedBy=multi-user.target
EOL

systemctl enable tycho-wallet-api-create-fs
