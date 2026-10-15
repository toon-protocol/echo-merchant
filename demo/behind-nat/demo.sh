#!/bin/bash
# An app behind NAT, with no open port, answers a paid request from another network.
#
#   ./demo.sh run        everything below, in order, then down
#   ./demo.sh up         start the provider VM and the Linode payer
#   ./demo.sh provider   install toon in the VM, fund it, start it, add the Echo Merchant
#   ./demo.sh pay        from the Linode: describe, fund, open a channel, pay
#   ./demo.sh evidence   what listens in the VM, and a port scan of the home from the Linode
#   ./demo.sh down       delete the Linode and the VM
#
# Devnet only: test USDC from the faucet. See README.md.
set -euo pipefail
source "$(dirname "$0")/lib/common.sh"
source "$DEMO_DIR/lib/vm.sh"
source "$DEMO_DIR/lib/linode.sh"

cmd_up() {
    say "The provider: a VM on this machine, behind QEMU's user-mode NAT, no port forwarded"
    vm_create
    say "The payer: a Linode Nanode on the public internet, tagged $LINODE_TAG"
    linode_create
    vm_wait
    linode_wait
    ssh_payer 'bash -s' -- "$NODE_VERSION" "$TOON_CLIENT_VERSION" < "$DEMO_DIR/guest/payer-setup.sh" 2>/dev/null | tail -1
    curl -4 -fsS https://ifconfig.me > "$HOME_IP_FILE"
    chmod 600 "$HOME_IP_FILE"
}

cmd_provider() {
    say "Provider: install toon $TOON_VERSION"
    vm "curl -fsSL https://toon.ar.io/install.sh | TOON_VERSION=$TOON_VERSION sh >/dev/null 2>&1; toon --version"
    say "Provider: create the agent node, a hidden service by default"
    # init prints the mnemonic once. It goes to a 0600 file in the VM, never to this screen.
    vm 'umask 077; toon init --network devnet --accept-anyone-terms --json > ~/init.json 2>/dev/null; jq -r .mnemonic ~/init.json > ~/.config/toon/mnemonic; jq "del(.mnemonic)" ~/init.json > ~/init.tmp && mv ~/init.tmp ~/init.json; jq "{network, toon_apps: [.toon_apps[] | {name, reach, onion_endpoint}]}" ~/init.json'
    vm_quiet 'cat ~/init.json' | python3 -c 'import json,sys; print(json.load(sys.stdin)["toon_apps"][0]["onion_endpoint"])' > "$STATE/onion"

    say "Provider: fund it from the devnet faucet and start it"
    vm 'toon wallet fund 2>&1 | head -3'
    vm 'toon up'
    vm_quiet 'for i in $(seq 60); do toon status >/dev/null 2>&1 && exit 0; sleep 2; done; exit 1' >/dev/null
    vm 'toon status'

    say "Provider: put the Echo Merchant behind the connector, at price 2"
    vm "toon add echo --to relay --image $ECHO_IMAGE --env ECHO_MERCHANT_ADDR=0.0.0.0:3100 --price 2 --yes"
    vm 'toon route list'
    vm 'toon status | grep -E "^(Supervisor|TOON app|App echo)"'
    vm_quiet 'toon route list' |
        awk '/\.echo /{print $1}' > "$STATE/route"
}

cmd_pay() {
    local onion route
    onion=$(cat "$STATE/onion") route=$(cat "$STATE/route")
    local env="cd /root/payer; export TOON_CONNECTOR=http://$onion; P='--password-file /root/.toon/password';"
    say "Payer: where it is. A different network: a Linode on the public internet"
    payer 'curl -4 -fsS https://ifconfig.me; echo; hostname'

    say "Payer: read the provider's self-description through the Anyone overlay (free)"
    local i
    for i in 1 2 3 4 5; do
        payer "$env npx toon-client describe \$P 2>&1 | grep -v -E '^toon-client: (downloading|verified)'" && break
        echo "(the hidden service is still publishing its address; retrying in 30s)"
        sleep 30
    done

    say "Payer: test USDC from the devnet faucet, and a channel toward the provider (no gas)"
    payer "$env npx toon-client faucet \$P 2>&1 | grep -v '^toon-client:'"
    payer "$env npx toon-client channel open --deposit 100000 \$P 2>&1 | grep -v '^toon-client:'"

    say "Payer: one paid request to the Echo Merchant behind NAT"
    payer "$env npx toon-client send $route --json-body --body '{\"hello\":\"from a linode\"}' \$P 2>&1 | grep -v '^toon-client:'"

    say "Provider: the channel the payment arrived on"
    vm 'toon channel list'
}

cmd_evidence() {
    say "Provider VM: every listening TCP socket. All on loopback"
    side 'provider-vm#' 'ss -tlnp'
    vm_root 'ss -tlnp' | sed -E 's/,fd=[0-9]+//g'

    say "Host: the QEMU process holds no listening socket, so nothing forwards into the VM"
    side 'host$' "ss -tlnp | grep qemu || echo 'no listening socket'"
    ss -tlnp 2>/dev/null | grep qemu || echo 'no listening socket'

    say "Payer: scan the home's public address (masked here) from the Linode"
    local h; h=$(cat "$HOME_IP_FILE")
    side 'linode-payer#' "nmap -n -Pn -T4 --top-ports 1000 <home-ip>"
    ssh_payer "nmap -n -Pn -T4 --top-ports 1000 $h 2>&1 | tail -4" | mask
    side 'linode-payer#' "nmap -n -Pn -p 22,80,443,7100 <home-ip>"
    ssh_payer "nmap -n -Pn -p 22,80,443,7100 $h | grep -E 'PORT|/tcp'" | mask
}

cmd_down() {
    say "Tear down: the Linode payer, then the provider VM"
    linode_destroy
    vm_destroy
    rm -f "$STATE/onion" "$STATE/route" "$HOME_IP_FILE"
}

cmd_run() {
    trap 'cmd_down' EXIT
    cmd_up
    cmd_provider
    cmd_pay
    cmd_evidence
}

case ${1:-} in
    up | provider | pay | evidence | down | run) "cmd_$1" ;;
    *) sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
