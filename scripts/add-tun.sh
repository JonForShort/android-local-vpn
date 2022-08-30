#!/bin/bash -x

# create tun device and change state to 'up'.
sudo ip tuntap add name tun0 mode tun user $USER
sudo ip link set tun0 up

# save routing table before modifying it.
sudo iptables-save > iptables.bak

# route everything through tun device.
sudo ip route add 128.0.0.0/1 dev tun0
sudo ip route add 0.0.0.0/1 dev tun0
