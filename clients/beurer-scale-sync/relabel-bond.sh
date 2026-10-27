#!/bin/sh
# Keep the Beurer BF500's pairing across Bluetooth restarts.
#
# The BF500 pairs with the Bluetooth specification's debug key. BlueZ stores that pairing
# as key type 4 (debug) and the kernel refuses to load debug keys, so every restart would
# need a new pairing with SET pressed on the scale. The key itself is valid; relabelling it
# as type 3 (authenticated P-256) makes BlueZ load and reuse it.
#
# Usage: sudo ./relabel-bond.sh D8:0B:CB:5F:A5:2D
set -eu
[ $# -eq 1 ] || { echo "usage: sudo $0 <scale address>" >&2; exit 2; }
[ "$(id -u)" -eq 0 ] || { echo "run with sudo" >&2; exit 1; }
address=$(printf %s "$1" | tr a-f A-F)
set -- /var/lib/bluetooth/*/"$address"/info
[ -f "$1" ] || { echo "no pairing stored for $address; run 'beurer-scale-sync pair' first" >&2; exit 1; }
# bluetoothd rewrites the file while running, so edit it only while stopped.
systemctl stop bluetooth
for info in "$@"; do
    sed -i 's/^Authenticated=4[[:space:]]*$/Authenticated=3/' "$info"
done
grep -H '^Authenticated=' "$@"
systemctl start bluetooth
