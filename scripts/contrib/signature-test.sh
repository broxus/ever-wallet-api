#!/usr/bin/env bash

function create_signature() {
  stringToSign="$1"
  echo -en "$stringToSign" | openssl sha256 -hmac "$secret" -binary | base64
}

timestamp="1704119227000"
sender="0:9b368ce8cd5aee1009a68d9946fd846812a7588c4a29b0727644cacea7c10f84"
recipient="0:82d6884271fab6516973024db8247c807f56085c99526d965d4bae695885f969"
amount="100000000"

uri="/ton/v3/transactions/create"
body='{"id": "", "fromAddress": "", "outputs": [{ "recipientAddress": "", "value": "" }]}'
body=$(echo "$body" | jq --indent 4 -r --arg id "14f7e109-9ada-4f36-9f79-9f08b4441a7b" '.id = $id')
body=$(echo "$body" | jq --indent 4 -r --arg sender  '.fromAddress = $sender')
body=$(echo "$body" | jq --indent 4 -r --arg recipient  '.outputs[0].recipientAddress = $recipient')
body=$(echo "$body" | jq --indent 4 -r --arg amount "" '.outputs[0].value = $amount')

echo $body

stringToSign="$timestamp$uri$body"
signature=$(create_signature "$stringToSign")

echo $stringToSign
echo $signature
