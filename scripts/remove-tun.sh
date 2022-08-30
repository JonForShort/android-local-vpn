#!/bin/bash -x

# delete tun device.
sudo ip link delete tun0

# restore ip tables.
sudo iptables-restore < iptables.bak
