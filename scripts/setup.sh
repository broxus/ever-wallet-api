#!/usr/bin/env bash
set -eE

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
REPO_DIR=$(cd "${SCRIPT_DIR}/../" && pwd -P)

function print_help() {
  echo 'Usage: setup.sh [OPTIONS]'
  echo ''
  echo 'Options:'
  echo '  -h,--help         Print this help message and exit'
  echo '  -t,--type TYPE    Installation types: native'
  echo '  --database-url    Postgres connection url which is needed to create'
  echo '                    database and make migration before running ton-wallet-api.'
  echo '                    example: "postgresql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}/${DB_NAME}"'
}

while [[ $# -gt 0 ]]; do
  key="$1"
  case $key in
      -h|--help)
        print_help
        exit 0
      ;;
      -t|--type)
        setup_type="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected installation type'
          echo ''
          print_help
          exit 1
        fi
      ;;
      --database-url)
        database_url="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected postgres connection url'
          echo ''
          print_help
          exit 1
        fi
      ;;
      *) # unknown option
        echo 'ERROR: Unknown option'
        echo ''
        print_help
        exit 1
      ;;
  esac
done

if [[ "$setup_type" != "native" ]]; then
  echo 'ERROR: Unknown installation type'
  echo ''
  print_help
  exit 1
fi

service_path="/etc/systemd/system/ton-wallet-api.service"
config_path="/etc/ton-wallet-api/config.yaml"

if [[ "$setup_type" == "native" ]]; then
  echo 'INFO: Running native installation'

  echo 'INFO: installing and updating dependencies'
  sudo apt update
  sudo apt install build-essential llvm clang curl libssl-dev pkg-config

  echo 'INFO: installing Rust'
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"

  echo 'INFO: installing sqlx-cli'
  cargo install sqlx-cli

  echo 'INFO: building ton-wallet-api'
  cd "$REPO_DIR"
  RUSTFLAGS="-C target_cpu=native" cargo build --release
  sudo cp "$REPO_DIR/target/release/ton-wallet-api" /usr/local/bin/ton-wallet-api

  echo 'INFO: creating systemd service'
  if [[ -f "$service_path" ]]; then
    echo "WARN: $service_path already exists"
  else
    sudo cp "$SCRIPT_DIR/contrib/ton-wallet-api.native.service" "$service_path"
  fi

else
  echo 'ERROR: Unexpected'
  exit 1
fi

echo "INFO: preparing environment"
sudo mkdir -p /etc/ton-wallet-api
sudo mkdir -p /var/db/ton-wallet-api
if [[ -f "$config_path" ]]; then
  echo "WARN: $config_path already exists"
else
  sudo cp -n "$SCRIPT_DIR/contrib/config.yaml" "$config_path"
fi
sudo curl -so /etc/ton-wallet-api/ton-global.config.json \
  https://raw.githubusercontent.com/tonlabs/main.ton.dev/master/configs/ton-global.config.json

echo 'INFO: restarting timesyncd'
sudo systemctl restart systemd-timesyncd.service

echo 'INFO: create database'
cargo sqlx database create --database-url "$database_url"

echo 'INFO: apply database migration'
cargo sqlx migrate run --database-url "$database_url"

echo 'INFO: done'
echo ''
echo 'INFO: Systemd service: ton-wallet-api'
echo '      Keys and configs: /etc/ton-wallet-api'
echo '      Node DB and stuff: /var/db/ton-wallet-api'
echo ''
echo 'NOTE: replace all "${..}" variables in /etc/ton-wallet-api/config.yaml'
echo '      or specify them in /etc/systemd/system/ton-wallet-api.service'
echo '      in "[Service]" section with something like this:'
echo '      Environment=SECRET=secret'
