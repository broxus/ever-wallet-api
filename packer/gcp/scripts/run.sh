#!/bin/bash

set -e

while true; do echo 'Waiting file system bean ready...'; if test -f /var/ton/ton-wallet-api/data/ready; then break; fi; sleep 5; done; echo 'File system is ready now';

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
. "${SCRIPT_DIR}/env.sh"

echo "INFO: apply database migrations..."

cd app
sqlx migrate run
cd ..

echo "INFO: start ton-wallet-api server..."

app/ton-wallet-api server --config app/config/config.yaml --global-config app/config/ton-global.config.json
