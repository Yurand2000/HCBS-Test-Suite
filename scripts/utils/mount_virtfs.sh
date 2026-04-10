#!/bin/sh

VM_FS=${1:?"Specify which virtio device to mount"}
VM_PATH=${2:?"Specify the mount point folder"}

if [ -n "$(mount | grep $VM_PATH)" ]; then
    echo "mount point folder already mounted"
    exit 1
fi

mkdir -p $VM_PATH
mount -t 9p -o trans=virtio $VM_FS "$VM_PATH" -oversion=9p2000.L
exit 0