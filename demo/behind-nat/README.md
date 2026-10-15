# An app behind NAT, paid from another network

**Live: Base and Solana mainnet · This demo: devnet, test USDC**

An Echo Merchant runs in a VM behind NAT. No port is forwarded and nothing listens for the
outside. A Linode on the public internet pays it one request and gets the answer.

[`demo.sh`](demo.sh) builds both machines from nothing, runs the payment, shows the evidence
and deletes both machines. [`recording/behind-nat.cast`](recording/behind-nat.cast) is one
full run.

```sh
asciinema play recording/behind-nat.cast
```

## The two machines

```
 this machine (behind a home router, behind the ISP's NAT)          the public internet
 ┌──────────────────────────────────────────────────────┐
 │ QEMU VM, user-mode NAT, no hostfwd                    │          ┌──────────────────┐
 │   toon agent node                                     │          │ Linode Nanode    │
 │     anon ── outbound circuits ────────────────────────┼──► Anyone overlay ◄── anon  │
 │     connector  127.0.0.1 only                         │          │ toon-client      │
 │     echo-merchant  docker network, no published port  │          │ pays, reads the  │
 └──────────────────────────────────────────────────────┘          │ Mirror           │
                                                                   └──────────────────┘
```

- **The provider** is a Debian 13 VM under QEMU with user-mode networking. That is a NAT:
  the VM can open connections out, and nothing outside can open one in. The demo forwards no
  port, not even SSH. It drives the VM through the QEMU guest agent on a unix socket.
- **The payer** is a Linode Nanode, on a different network. It runs
  [`toon-client`](https://github.com/toon-protocol/toon-client) and nothing else of TOON.
- **The path** is the Anyone overlay. The provider's connector is a hidden service at a
  `.anyone` address. Both ends dial out to the overlay. Neither accepts a connection from
  the internet.

## Run it

You need Linux with KVM, `qemu-system-x86_64`, `python3`, `uv`, `ssh`, `curl`, `jq`,
foundry's `cast`, and `linode-cli` with a token that may create Linodes.

A fresh agent node needs Base Sepolia ETH for gas, to join the devnet and to land what it
is paid, until [toon_cli#255](https://github.com/toon-protocol/toon_cli/issues/255) is
fixed. The devnet faucet sends none. Give the demo a Base Sepolia wallet to send it from:

```sh
uv tool install linode-cli
export LINODE_CLI_TOKEN=...            # or LINODE_ACCESS_TOKEN
export GAS_FUNDER_MNEMONIC=...         # a Base Sepolia wallet; sends 0.002 ETH, never printed
./demo.sh run                          # about 8 minutes
```

Without `GAS_FUNDER_MNEMONIC` the demo prints the provider's address and waits until you
fund it from a public Base Sepolia faucet.

`run` is `up`, `provider`, `pay`, `settle`, `hop` and `evidence` in order, then `down`, which
runs even when a step fails. Each step also runs on its own.

| Step | What it does |
| --- | --- |
| `up` | Boots the VM from the Debian cloud image and creates one `g6-nanode-1`, tagged `toon-demo` |
| `provider` | In the VM: installs `toon` v0.2.0, `toon init`, `toon wallet fund`, gas, `toon up`, `toon join devnet`, `toon add echo` |
| `pay` | On the Linode: `toon-client describe`, `faucet`, `channel open`, `send` |
| `settle` | In the VM: `toon channel land`, then the claim's transaction hash, read off Base Sepolia |
| `hop` | In the VM: pays `g.toon.relay.gas` through the devnet hub, hop by hop |
| `evidence` | `ss -tlnp` in the VM and on the host, `nmap` of the home's public address from the Linode |
| `down` | Deletes the Linode this demo made, then stops and deletes the VM |

It costs a few cents of Linode time and 0.002 Base Sepolia ETH. The USDC is devnet USDC
from the faucet.

## What it prints

From the run in the recording, on 2026-10-15. The `.anyone` address, the routes and the
channels were new for that run. The machines are gone; the transactions are on Base Sepolia.

The provider gets gas, joins the devnet and adds the Echo Merchant at price 2:

```text
host$ cast send --mnemonic "$GAS_FUNDER_MNEMONIC" --value 0.002ether 0xAA078CfaB21404715AD618C5e8B10674CdA552Ca
funding tx 0x9c31ea796af4096ce13bcd31cc8af472ed042a630cf7ab1f1dc503d2bc8a1455 (status 0x1)
provider-vm$ toon join devnet --deposit 1000000 --yes
Joined devnet: peered with https://proxy.relay.devnet.toonprotocol.dev/ilp and forwarding g.toon to it, deposit 1000000. Reading its relay at wss://relay-ws.devnet.toonprotocol.dev.
Its connector forwards back to you only if its operator creates a peering toward you in return.
provider-vm$ toon route list
g.toon.151cc3609b6b5c30.relay -> http://127.0.0.1:61326/write (price 1)
g.toon.151cc3609b6b5c30.relay.ephemeral -> http://127.0.0.1:61326/write-ephemeral (price 0)
g.toon.151cc3609b6b5c30.echo -> http://172.18.0.2:3100/ (price 2)
g.toon -> peer devnet (price 0)
```

The payer, on the Linode, opens a channel to the provider's `.anyone` address and pays one
request:

```text
linode-payer# npx toon-client channel open --deposit 100000
Channel open.
  channel    0x60765c332b392b72e34107631d7ff898b66b16d0e1b2ddda72a78b3acae16c7d
  network    eip155:84532
  deposit    100000 (0.1 USDC)
linode-payer# npx toon-client send g.toon.151cc3609b6b5c30.echo --json-body --body '{"hello":"from a linode"}'
FULFILL 200  (http)
  paid        2 (0.000002 USDC)  channel 0x60765c332b392b72e34107631d7ff898b66b16d0e1b2ddda72a78b3acae16c7d
  cumulative  2 (0.000002 USDC)

{"payment":{"amount":"2","chain":"evm","payer":"evm:0x6076…6c7d"},"request":{"body":"{\"hello\":\"from a linode\"}","body_encoding":"utf8",…,"method":"POST","path":"/"}}
```

The provider lands the voucher on chain:

```text
provider-vm$ toon channel land 0x60765c332b392b72e34107631d7ff898b66b16d0e1b2ddda72a78b3acae16c7d
Landed: 0x6076…6c7d evm inbound open (counterparty 0xb01d…2217, collateral 99998, landed 2, watermark 2)
host$ the claim on Base Sepolia, from the provider's settlement key
0x9c2b9aaa99f92af25acce2bad0bb5cc0c5b8b825df10cd22d11570e831d62f3d  claim
```

[The claim on Basescan](https://sepolia.basescan.org/tx/0x9c2b9aaa99f92af25acce2bad0bb5cc0c5b8b825df10cd22d11570e831d62f3d).
`toon channel land` prints no transaction hash, so the demo reads it from the
x402BatchSettlement contract's logs.

From behind the same NAT, the provider pays a devnet route hop by hop, through the hub:

```text
provider-vm$ toon send g.toon.relay.gas --amount 1001 --seal-to https://proxy.gas.devnet.toonprotocol.dev/ilp --body ~/body.json --yes
Fulfilled: 1001 base units to g.toon.relay.gas. The app answered 422.
```

The packet went from this node to the devnet relay, and on to the gas station. The gas
station answered 422 because it wants a real gas request, not `{"hello":…}`. A 422 is still
a paid answer.

Nothing listens for the outside:

```text
provider-vm# ss -tlnp
LISTEN 0  4096  127.0.0.54:53      0.0.0.0:*  users:(("systemd-resolve",pid=2118))
LISTEN 0  4096 127.0.0.53%lo:53    0.0.0.0:*  users:(("systemd-resolve",pid=2118))
LISTEN 0  4096   127.0.0.1:33671   0.0.0.0:*  users:(("containerd",pid=1730))
LISTEN 0  128    127.0.0.1:61326   0.0.0.0:*  users:(("relay",pid=2858))
LISTEN 0  4096   127.0.0.1:61044   0.0.0.0:*  users:(("anon",pid=2787))
LISTEN 0  128    127.0.0.1:65186   0.0.0.0:*  users:(("relay",pid=2858))
LISTEN 0  128    127.0.0.1:22261   0.0.0.0:*  users:(("toon",pid=3162))

host$ ss -tlnp | grep qemu || echo 'no listening socket'
no listening socket

linode-payer# nmap -n -Pn -T4 --top-ports 1000 <home-ip>
All 1000 scanned ports on <home-ip> are in ignored states.
Not shown: 1000 filtered tcp ports (no-response)
```

The connector listens on `127.0.0.1:22261`. The Echo Merchant listens on a Docker network
inside the VM and publishes no port. The only way in is the `anon` daemon's outbound
circuits.

## What it shows, and what it does not

- The app answered a request that came from a different network. The request was paid: the
  Mirror's `payment` is the payer's channel and the charge, 2 base units. The provider landed
  that voucher on Base Sepolia.
- Nothing listened for the outside. In the VM every listening socket is on `127.0.0.1`. On
  the host, QEMU holds no listening socket. From the Linode, every scanned port on the home's
  public address is filtered.
- The paid request into the provider goes straight to its connector, not through the hub.
  `join` makes a peering one way, from the provider to the hub. The hub's connector learns no
  routes, so it forwards to the provider only if the hub's operator peers toward it. The
  demo shows the other direction instead: the node behind NAT pays out through the hub.
- A hidden service hides where the app is reachable. It does not hide who pays: the channel
  is on a public chain.

## Keys and secrets

The provider's mnemonic and passphrase are made in the VM and stay there. The payer's are
made on the Linode and stay there. Both are deleted with their machine. The Linode root
password is random and never shown; login is by an SSH key made in `.state/`. The home's
public address is masked as `<home-ip>` in everything the demo prints. The Linode token is
passed to `linode-cli` in its environment and never printed. `GAS_FUNDER_MNEMONIC` is read
by `cast` from the environment, signs on Base Sepolia only, and is never printed.

## Safety

`down` deletes only an instance whose id this demo recorded, or whose label starts
`toon-demo-payer-`, and only if it carries the `toon-demo` tag. It touches nothing else on
the account.

## Found on the way

- [toon_cli#255](https://github.com/toon-protocol/toon_cli/issues/255): `toon join` needs
  Base Sepolia ETH on a fresh node. The demo sends 0.002 ETH from `GAS_FUNDER_MNEMONIC`.
- [toon_cli#256](https://github.com/toon-protocol/toon_cli/issues/256): `toon status` right
  after `toon up` says the supervisor is not running. The demo waits for it.
- [toon-client#745](https://github.com/toon-protocol/toon-client/issues/745): the managed
  `anon` daemon needs `unzip`. The demo installs it on the Linode.
- The devnet relay does not forward `g.toon.echo`. Its `GET /ilp` lists `g.toon.relay`,
  `.ephemeral`, `.gas` and `.store`, and a joined node's `toon send g.toon.echo` is
  rejected `F02`. [The Echo flow](../../docs/guide/echo-flow.md) says joining reaches it. The
  demo's hop uses `g.toon.relay.gas`.
