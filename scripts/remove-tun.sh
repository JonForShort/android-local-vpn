#!/bin/bash -x

# current directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"

main() {
    pushd ${SCRIPT_DIR}
    
    # delete tun device.
    sudo ip link delete tun0
    
    # restore ip tables.
    sudo iptables-restore < iptables.bak
    
    popd
}

main
