#!/usr/bin/env bash

function print_help() {
  echo 'Usage: wallet.sh [OPTIONS]'
  echo ''
  echo 'Options:'
  echo '  -h,--help         Print this help message and exit'
  echo '  -m, --method      Wallet actions:'
  echo ''
  echo '                      - create_account - create HighWallet account.'
  echo ''
  echo '                      - create_transaction - create TON transaction.'
  echo '                        Options:'
  echo '                          --src-addr    Sender address'
  echo '                          --dst-addr    Recipient address'
  echo '                          --amount      Ton amount'
  echo ''
  echo '                      - create_token_transaction - create Token transaction.'
  echo '                        Options:'
  echo '                          --src-addr    Sender address'
  echo '                          --dst-addr    Recipient address'
  echo '                          --root-addr   Root Token address'
  echo '                          --amount      Token amount'
}

while [[ $# -gt 0 ]]; do
  key="$1"
  case $key in
      -h|--help)
        print_help
        exit 0
      ;;
      -m|--method)
        method="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected method'
          echo ''
          print_help
          exit 1
        fi
      ;;
      --src-addr)
        sender="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected sender'
          echo ''
          print_help
          exit 1
        fi
      ;;
      --dst-addr)
        recipient="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected recipient'
          echo ''
          print_help
          exit 1
        fi
      ;;
      --amount)
        amount="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected amount'
          echo ''
          print_help
          exit 1
        fi
      ;;
      --root-addr)
        root_address="$2"
        shift # past argument
        if [ "$#" -gt 0 ]; then shift;
        else
          echo 'ERROR: Expected token root'
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

function fail {	echo "$1" > /dev/stderr; exit 1; }

if [[ -z "$API_KEY" ]]; then fail "API_KEY env is not set"
else
  api_key="$API_KEY"
fi

if [[ -z "$SECRET" ]]; then fail "SECRET env is not set"
else
  secret="$SECRET"
fi

if [[ -z "$HOST" ]]; then fail "HOST env is not set"
else
  host="$HOST"
fi

function timestamp_ms {
  date +%s000
}

function create_signature() {
  stringToSign="$1"
  echo -en "$stringToSign" | openssl sha256 -hmac "$secret" -binary | base64
}

function create_account() {
  timestamp=$1
  uri="/ton/v3/address/create"
  body="{}"

  stringToSign="$timestamp$uri$body"
  signature=$(create_signature "$stringToSign")

  curl -s --location --request POST "$host$uri" \
    --header 'Content-Type: application/json' \
    --header "api-key: $api_key" \
    --header "timestamp: $timestamp" \
    --header "sign: $signature" \
    --data-raw "$body"
}

function create_transaction() {
  timestamp=$1
  sender=$2
  recipient=$3
  amount=$4

  uri="/ton/v3/transactions/create"
  body='{"id": "", "fromAddress": "", "outputs": [{ "recipientAddress": "", "value": "" }]}'
  body=$(echo "$body" | jq --indent 4 -r --arg id "$(uuidgen)" '.id = $id')
  body=$(echo "$body" | jq --indent 4 -r --arg sender "$sender" '.fromAddress = $sender')
  body=$(echo "$body" | jq --indent 4 -r --arg recipient "$recipient" '.outputs[0].recipientAddress = $recipient')
  body=$(echo "$body" | jq --indent 4 -r --arg amount "$amount" '.outputs[0].value = $amount')

  stringToSign="$timestamp$uri$body"
  signature=$(create_signature "$stringToSign")

  curl -s --location --request POST "$host$uri" \
    --header 'Content-Type: application/json' \
    --header "api-key: $api_key" \
    --header "timestamp: $timestamp" \
    --header "sign: $signature" \
    --data-raw "$body"
}

function create_token_transaction() {
  timestamp=$1
  sender=$2
  recipient=$3
  root_address=$4
  amount=$5

  uri="/ton/v3/tokens/transactions/create"
  body='{"id": "", "fromAddress": "", "outputs": [{ "recipientAddress": "", "value": "" }]}'
  body=$(echo "$body" | jq --indent 4 -r --arg id "$(uuidgen)" '.id = $id')
  body=$(echo "$body" | jq --indent 4 -r --arg sender "$sender" '.fromAddress = $sender')
  body=$(echo "$body" | jq --indent 4 -r --arg recipient "$recipient" '.recipientAddress = $recipient')
  body=$(echo "$body" | jq --indent 4 -r --arg root_address "$root_address" '.rootAddress = $root_address')
  body=$(echo "$body" | jq --indent 4 -r --arg amount "$amount" '.value = $amount')

  stringToSign="$timestamp$uri$body"
  signature=$(create_signature "$stringToSign")

  curl -s --location --request POST "$host$uri" \
    --header 'Content-Type: application/json' \
    --header "api-key: $api_key" \
    --header "timestamp: $timestamp" \
    --header "sign: $signature" \
    --data-raw "$body"
}

case $method in
  create_account)
    timestamp=$(timestamp_ms)
    create_account "$timestamp" | jq .
  ;;
  create_transaction)
    if [ -z "$sender" ]; then
      echo 'ERROR: Skipped sender'
      echo ''
      print_help
      exit 1
    fi
    if [ -z "$recipient" ]; then
      echo 'ERROR: Skipped recipient'
      echo ''
      print_help
      exit 1
    fi
    if [ -z "$amount" ]; then
      echo 'ERROR: Skipped amount'
      echo ''
      print_help
      exit 1
    fi

    timestamp=$(timestamp_ms)
    create_transaction "$timestamp" "$sender" "$recipient" "$amount" | jq .
  ;;
  create_token_transaction)
    if [ -z "$sender" ]; then
      echo 'ERROR: Skipped sender'
      echo ''
      print_help
      exit 1
    fi
    if [ -z "$recipient" ]; then
      echo 'ERROR: Skipped recipient'
      echo ''
      print_help
      exit 1
    fi
    if [ -z "$root_address" ]; then
      echo 'ERROR: Skipped token root address'
      echo ''
      print_help
      exit 1
    fi
    if [ -z "$amount" ]; then
      echo 'ERROR: Skipped amount'
      echo ''
      print_help
      exit 1
    fi

    timestamp=$(timestamp_ms)
    create_token_transaction "$timestamp" "$sender" "$recipient" "$root_address" "$amount" | jq .
  ;;
  *) # unknown method
    echo 'ERROR: Unknown method'
    echo ''
    print_help
    exit 1
  ;;
esac
