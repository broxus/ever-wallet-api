#!/usr/bin/env bash
set -eE

function print_help() {
  echo 'Usage: root_token.sh [OPTIONS] [PATH]'
  echo ''
  echo 'Options:'
  echo '  -h,--help         Print this help message and exit'
  echo '  -t,--type TYPE    Installation type: native'
  echo '  --database-url    Postgres connection url which is needed to create'
  echo '                    database and make migration before running ton-wallet-api.'
  echo '                    example: "postgresql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}/${DB_NAME}"'
  echo '  --name            Token name (ticker)'
  echo '  --address         Token address'
  echo '  --version         Token version: Tip3 | OldTip3v4'
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
      --name)
        token_name="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected token name'
          echo ''
          print_help
          exit 1
        fi
      ;;
      --address)
        token_address="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected token address'
          echo ''
          print_help
          exit 1
        fi
      ;;
      --version)
        version="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected token version'
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
  ton_wallet_api_binary="/usr/local/bin/ton-wallet-api root_token"
else
  echo 'ERROR: Unexpected'
  exit 1
fi

if [[ $version != "Tip3" ]] && [[ $version != "OldTip3v4" ]]; then
  echo 'ERROR: Invalid token version'
  exit 1
fi

sudo -E bash -c "DATABASE_URL=$database_url $ton_wallet_api_binary --name $token_name --address $token_address --version $version"
