#!/bin/bash

set -eux

echo "INFO: check file system..."

sleep 60;

if ! test -f /var/ton/ton-wallet-api/data/ready; then
  echo "INFO: create raid..."
  mdadm --create --verbose /dev/md0 --level=0 --raid-devices=4 /dev/nvme0n1 /dev/nvme0n2 /dev/nvme0n3 /dev/nvme0n4
  mkfs.ext4 -F /dev/md0
  mount /dev/md0 /var/ton/ton-wallet-api
  mkdir /var/ton/ton-wallet-api/data
  touch /var/ton/ton-wallet-api/data/ready
  chown -R ton:ton /var/ton/ton-wallet-api/data
  echo '/dev/md0 /var/ton/ton-wallet-api ext4 defaults,nofail,discard 0 0' | tee -a /etc/fstab
fi

echo "INFO: file system is ready now"
