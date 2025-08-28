#!/bin/bash

set -e

while true; do echo 'Waiting file system bean ready...'; if test -f /var/ton/tycho-wallet-api/data/ready; then break; fi; sleep 5; done; echo 'File system is ready now';

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
. "${SCRIPT_DIR}/env.sh"

echo "INFO: apply database migrations..."

cd app
sqlx migrate run
cd ..

echo "INFO: start tycho-wallet-api server..."

app/tycho-wallet-api server --config app/config/config.json --global-config app/config/global-config.json --keys app/config/keys.json
