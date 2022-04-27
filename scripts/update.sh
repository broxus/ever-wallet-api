#!/usr/bin/env bash
set -eE

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
REPO_DIR=$(cd "${SCRIPT_DIR}/../" && pwd -P)

function print_help() {
  echo 'Usage: update.sh [OPTIONS]'
  echo ''
  echo 'Options:'
  echo '  -h,--help         Print this help message and exit'
  echo '  -f,--force        Clear "/var/db/ton-wallet-api" on update'
  echo '  -s,--sync         Restart "timesyncd" service'
  echo '  -t,--type TYPE    Installation types: native'
  echo '  --database-url    Postgres connection url which is needed to create'
  echo '                    database and make migration before running ton-wallet-api.'
  echo '                    example: "postgresql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}/${DB_NAME}"'
}

force="false"
restart_timesyncd="false"
while [[ $# -gt 0 ]]; do
  key="$1"
  case $key in
      -h|--help)
        print_help
        exit 0
      ;;
      -f|--force)
        force="true"
        shift # past argument
      ;;
      -s|--sync)
        restart_timesyncd="true"
        shift # past argument
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

echo "INFO: stopping ton-wallet-api service"
sudo systemctl stop ton-wallet-api

if [[ "$force" == "true" ]]; then
  echo "INFO: removing ton-wallet-api db"
  sudo rm -rf /var/db/ton-wallet-api
else
  echo 'INFO: skipping "/var/db/ton-wallet-api" deletion'
fi

if [[ "$setup_type" == "native" ]]; then
  echo 'INFO: running update for native installation'

  source "$HOME/.cargo/env"

  echo 'INFO: building ton-wallet-api'
  cd "$REPO_DIR"
  RUSTFLAGS="-C target_cpu=native" cargo build --release
  sudo cp "$REPO_DIR/target/release/ton-wallet-api" /usr/local/bin/ton-wallet-api

else
  echo 'ERROR: Unexpected'
  exit 1
fi

echo "INFO: preparing environment"
sudo mkdir -p /var/db/ton-wallet-api

if [[ "$restart_timesyncd" == "true" ]]; then
  echo 'INFO: restarting timesyncd'
  sudo systemctl restart systemd-timesyncd.service
fi

echo 'INFO: apply database migration'
cargo sqlx migrate run --database-url "$database_url"

echo 'INFO: restarting ton-wallet-api service'
sudo systemctl restart ton-wallet-api

echo 'INFO: done'
echo ''
echo 'INFO: Systemd service: ton-wallet-api'
echo '      Keys and configs: /etc/ton-wallet-api'
echo '      Node DB and stuff: /var/db/ton-wallet-api'
echo ''
