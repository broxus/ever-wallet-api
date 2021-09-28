#!/bin/bash

set -e

SERVICE_DIR=app
SERVICE=ton-wallet-api
USER=ton
GROUP=ton

UPSTREAM=`curl --retry 5 -sfH "Metadata-Flavor: Google" "http://metadata/computeMetadata/v1/instance/attributes/upstream"`

CURRENT_VERSION=`cat updates/current || true`
NEW_VERSION=`gsutil ls $UPSTREAM | sort | tail -n 1 | cut -d '/' -f 7`

echo "INFO: check new version, current version is $CURRENT_VERSION"

RUN_SCRIPT=`cat run.sh`
echo "$RUN_SCRIPT"

if [ "$CURRENT_VERSION" != "$NEW_VERSION" ]; then

  echo "INFO: new version is available, $NEW_VERSION, start downloading..."

  UPDATE_DIR="updates/$NEW_VERSION"

  mkdir -p $UPDATE_DIR
  gsutil cp -r "$UPSTREAM/$NEW_VERSION/*" $UPDATE_DIR

  systemctl stop $SERVICE
  rm -rf $SERVICE_DIR || true
  mkdir -p $SERVICE_DIR
  cp -r $UPDATE_DIR/* $SERVICE_DIR
  chmod +x $SERVICE_DIR/$SERVICE
  chown -R $USER:$GROUP $SERVICE_DIR
  systemctl start $SERVICE

  echo "$NEW_VERSION" > updates/current
fi
