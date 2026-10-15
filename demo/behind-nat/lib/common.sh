# Shared settings and helpers. Sourced by demo.sh.

DEMO_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
STATE=${TOON_DEMO_STATE:-$DEMO_DIR/.state}
mkdir -p "$STATE" && chmod 700 "$STATE"

# The provider VM.
VM_IMAGE_URL=${VM_IMAGE_URL:-https://cloud.debian.org/images/cloud/trixie/latest/debian-13-genericcloud-amd64.qcow2}
VM_MEMORY=${VM_MEMORY:-4096}
VM_CPUS=${VM_CPUS:-2}
VM_DISK=${VM_DISK:-20G}
QGA_SOCK=$STATE/qga.sock

# The payer on Linode. Every resource this demo makes carries LINODE_TAG.
LINODE_TAG=toon-demo
LINODE_LABEL=${LINODE_LABEL:-toon-demo-payer}
LINODE_TYPE=${LINODE_TYPE:-g6-nanode-1}
LINODE_REGION=${LINODE_REGION:-us-east}
LINODE_IMAGE=${LINODE_IMAGE:-linode/debian13}

# Versions, pinned.
TOON_VERSION=${TOON_VERSION:-v0.2.0}
TOON_CLIENT_VERSION=${TOON_CLIENT_VERSION:-5.0.1}
NODE_VERSION=${NODE_VERSION:-v22.20.0}
ECHO_IMAGE=${ECHO_IMAGE:-ghcr.io/toon-protocol/echo-merchant:release}

export PATH=$HOME/.local/bin:$PATH

# linode-cli reads LINODE_CLI_TOKEN. Map LINODE_ACCESS_TOKEN onto it in the
# child's environment only; the token is never printed.
lin() {
    LINODE_CLI_TOKEN="${LINODE_CLI_TOKEN:-${LINODE_ACCESS_TOKEN:-}}" \
        linode-cli --no-defaults --suppress-warnings "$@"
}

say()  { printf '\n\033[1;33m# %s\033[0m\n' "$*"; }
side() { printf '\033[1;36m%s\033[0m %s\n' "$1" "$2"; }
die()  { printf 'demo: %s\n' "$*" >&2; exit 1; }

# The home's public address is masked in everything this demo prints.
HOME_IP_FILE=$STATE/home-ip
mask() {
    if [ -s "$HOME_IP_FILE" ]; then sed "s/$(sed 's/\./\\./g' "$HOME_IP_FILE")/<home-ip>/g"; else cat; fi
}

# Run a command in the provider VM, as the toon user, and show it.
vm() {
    side 'provider-vm$' "$*"
    python3 "$DEMO_DIR/lib/qga.py" "$QGA_SOCK" exec --as toon -- "$@" 2>&1 | mask
    return "${PIPESTATUS[0]}"
}
vm_quiet() { python3 "$DEMO_DIR/lib/qga.py" "$QGA_SOCK" exec --as toon -- "$@"; }
vm_root() { python3 "$DEMO_DIR/lib/qga.py" "$QGA_SOCK" exec -- "$@"; }

# Run a command on the Linode payer, and show it.
SSH_KEY=$STATE/id_ed25519
payer_ip() { cat "$STATE/linode-ip"; }
ssh_payer() {
    ssh -i "$SSH_KEY" -o UserKnownHostsFile="$STATE/known_hosts" -o StrictHostKeyChecking=accept-new \
        -o ConnectTimeout=10 -o LogLevel=ERROR "root@$(payer_ip)" "$@"
}
payer() {
    side 'linode-payer#' "$*"
    ssh_payer "$@" 2>&1 | mask
    return "${PIPESTATUS[0]}"
}
