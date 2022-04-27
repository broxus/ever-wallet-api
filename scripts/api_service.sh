#!/usr/bin/env bash
set -eE

function print_help() {
  echo 'Usage: api_service.sh [OPTIONS] [PATH]'
  echo ''
  echo 'Options:'
  echo '  -h,--help         Print this help message and exit'
  echo '  -t,--type TYPE    Installation type: native'
  echo '  --database-url    Postgres connection url which is needed to create'
  echo '                    database and make migration before running ton-wallet-api.'
  echo '                    example: "postgresql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}/${DB_NAME}"'
  echo '  --id              Service id (UUID)'
  echo '  --name            Service name'
  echo '  --key             HMAC public key'
  echo '  --secret          HMAC secret key'
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
      --id)
        service_id="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected service id'
          echo ''
          print_help
          exit 1
        fi
      ;;
      --name)
        service_name="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected service name'
          echo ''
          print_help
          exit 1
        fi
      ;;
      --key)
        service_key="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected service key'
          echo ''
          print_help
          exit 1
        fi
      ;;
      --secret)
        service_secret="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected service secret'
          echo ''
          print_help
          exit 1
        fi
      ;;
      *)    # unknown option
      path="$1"
      shift # past argument
      ;;
  esac
done

if [[ "$setup_type" == "native" ]]; then
  ton_wallet_api_binary="/usr/local/bin/ton-wallet-api api_service"
else
  echo 'ERROR: Unexpected'
  exit 1
fi

echo "Exporting keys from $path"
sudo -E bash -c "DATABASE_URL=$database_url $ton_wallet_api_binary --id $service_id --name $service_name --key service_key --secret $service_secret"
