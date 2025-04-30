#!/bin/bash

nvme_drive_size=$input
fdisk -l
df -h
mkdir -p /mnt/nvme_disk
nvme_drive=$(sudo fdisk -l | grep $nvme_drive_size | awk '{print $2}' | sed -e 's/://g')
# IF THE VALUE OF NVME_DRIVE IS EMPTY OR HAS MORE THAN 1 HITS, ABORT THE PROGRAM WITH A HELPFUL ERROR MESSAGE
echo -e "o\nn\np\n1\n\n\nw" | sudo fdisk $nvme_drive
mkfs.ext4 ${nvme_drive}1
mount -t ext4 ${nvme_drive}1 /mnt/nvme_disk
fdisk -l

