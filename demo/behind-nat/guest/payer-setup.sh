#!/bin/bash
# Runs on the Linode payer as root: Node.js, toon-client and nmap.
set -euo pipefail
NODE_VERSION=$1 TOON_CLIENT_VERSION=$2
export DEBIAN_FRONTEND=noninteractive
apt-get -qq update && apt-get -qq install -y nmap jq curl xz-utils unzip >/dev/null
if ! command -v node >/dev/null; then
    curl -fsSL "https://nodejs.org/dist/$NODE_VERSION/node-$NODE_VERSION-linux-x64.tar.xz" | tar -xJ -C /usr/local --strip-components=1
fi
mkdir -p /root/payer && cd /root/payer
[ -f package.json ] || npm init -y >/dev/null
npm install --silent --no-fund --no-audit "@toon-protocol/client@$TOON_CLIENT_VERSION"
umask 077
mkdir -p /root/.toon
[ -s /root/.toon/password ] || head -c 32 /dev/urandom | base64 > /root/.toon/password
# init prints the phrase once; it stays on this box, in a 0600 file.
[ -s /root/.toon/keystore.json ] || npx toon-client init --password-file /root/.toon/password > /root/.toon/init.out 2>&1
echo "node $(node --version), toon-client $(npx toon-client --version), $(nmap --version | head -1)"
