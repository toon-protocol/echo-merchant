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

You need Linux with KVM, `qemu-system-x86_64`, `python3`, `uv`, `ssh`, `curl`, and `linode-cli` with a
token that may create Linodes.

```sh
uv tool install linode-cli
export LINODE_CLI_TOKEN=...            # or LINODE_ACCESS_TOKEN
./demo.sh run                          # about 6 minutes
```

`run` is `up`, `provider`, `pay` and `evidence` in order, then `down`, which runs even when a
step fails. Each step also runs on its own.

| Step | What it does |
| --- | --- |
| `up` | Boots the VM from the Debian cloud image and creates one `g6-nanode-1`, tagged `toon-demo` |
| `provider` | In the VM: installs `toon` v0.2.0, `toon init`, `toon wallet fund`, `toon up`, `toon add echo` |
| `pay` | On the Linode: `toon-client describe`, `faucet`, `channel open`, `send` |
| `evidence` | `ss -tlnp` in the VM and on the host, `nmap` of the home's public address from the Linode |
| `down` | Deletes the Linode this demo made, then stops and deletes the VM |

It costs a few cents of Linode time. The payer's deposit is 0.1 devnet USDC from the faucet.

## What it prints

From the run in the recording, on 2026-10-15. The `.anyone` address, the routes and the
channel were new for that run and are gone now.

The provider creates a hidden service, adds the Echo Merchant at price 2, and starts:

```text
provider-vm$ toon route list
g.toon.245af228762c9581.relay -> http://127.0.0.1:62445/write (price 1)
g.toon.245af228762c9581.relay.ephemeral -> http://127.0.0.1:62445/write-ephemeral (price 0)
g.toon.245af228762c9581.echo -> http://172.18.0.2:3100/ (price 2)
provider-vm$ toon status | grep -E "^(Supervisor|TOON app|App echo)"
Supervisor: running.
TOON app relay (ILP address g.toon.245af228762c9581): connector running on 127.0.0.1:28872.
App echo of relay: running on 172.18.0.2:3100. Read at ws://172.18.0.2:7100. Route g.toon.245af228762c9581.echo at price 2.
```

The payer, on the Linode, opens a channel and pays one request:

```text
linode-payer# npx toon-client channel open --deposit 100000
Channel open.
  channel    0x751d362225b3b9449dd0b4da4c9aded821470bc267c9dcec9da1d6ed3bbac164
  network    eip155:84532
  deposit    100000 (0.1 USDC)
linode-payer# npx toon-client send g.toon.245af228762c9581.echo --json-body --body '{"hello":"from a linode"}'
FULFILL 200  (http)
  paid        2 (0.000002 USDC)  channel 0x751d362225b3b9449dd0b4da4c9aded821470bc267c9dcec9da1d6ed3bbac164
  cumulative  2 (0.000002 USDC)

{"payment":{"amount":"2","chain":"evm","payer":"evm:0x751d…c164"},"request":{"body":"{\"hello\":\"from a linode\"}","body_encoding":"utf8",…,"method":"POST","path":"/"}}
```

The provider sees the payment arrive on that channel:

```text
provider-vm$ toon channel list
0x751d…c164 evm inbound open (counterparty 0x19f6…4186, collateral 100000, landed 0, watermark 2)
```

Nothing listens for the outside:

```text
provider-vm# ss -tlnp
LISTEN 0  4096   127.0.0.1:44963   0.0.0.0:*  users:(("containerd",pid=1739))
LISTEN 0  128    127.0.0.1:62445   0.0.0.0:*  users:(("relay",pid=2455))
LISTEN 0  4096   127.0.0.1:62028   0.0.0.0:*  users:(("anon",pid=2386))
LISTEN 0  4096  127.0.0.54:53      0.0.0.0:*  users:(("systemd-resolve",pid=2129))
LISTEN 0  128    127.0.0.1:63844   0.0.0.0:*  users:(("relay",pid=2455))
LISTEN 0  4096 127.0.0.53%lo:53    0.0.0.0:*  users:(("systemd-resolve",pid=2129))
LISTEN 0  128    127.0.0.1:28872   0.0.0.0:*  users:(("toon",pid=2655))

host$ ss -tlnp | grep qemu || echo 'no listening socket'
no listening socket

linode-payer# nmap -n -Pn -T4 --top-ports 1000 <home-ip>
All 1000 scanned ports on <home-ip> are in ignored states.
Not shown: 1000 filtered tcp ports (no-response)
```

The connector listens on `127.0.0.1:28872`. The Echo Merchant listens on a Docker network
inside the VM and publishes no port. The only way in is the `anon` daemon's outbound
circuits.

## What it shows, and what it does not

- The app answered a request that came from a different network. The request was paid:
  the Mirror's `payment` is the payer's channel and the charge, 2 base units.
- Nothing listened for the outside. In the VM every listening socket is on `127.0.0.1`.
  On the host, QEMU holds no listening socket. From the Linode, every scanned port on the
  home's public address is filtered.
- The payment is a signed voucher on a channel funded on Base Sepolia. The provider has not
  settled it on chain. Settling costs the provider gas, which the devnet faucet does not
  send.
- A hidden service hides where the app is reachable. It does not hide who pays: the channel
  is on a public chain.

## Keys and secrets

The provider's mnemonic and passphrase are made in the VM and stay there. The payer's are
made on the Linode and stay there. Both are deleted with their machine. The Linode root
password is random and never shown; login is by an SSH key made in `.state/`. The home's
public address is masked as `<home-ip>` in everything the demo prints. The Linode token is
passed to `linode-cli` in its environment and never printed.

## Safety

`down` deletes only an instance whose id this demo recorded, or whose label starts
`toon-demo-payer-`, and only if it carries the `toon-demo` tag. It touches nothing else on
the account.

## Why there is no `toon join`

The documented first run has `toon join devnet --deposit 1000000 --yes` after `toon up`. This
demo leaves it out. `join` opens a channel toward the devnet hub, and on a fresh agent node it
stops with "The settlement key of relay has no gas for a transaction": it needs 0.001 Base
Sepolia ETH, which the faucet does not send. The payer here pays the provider's connector
directly, so the provider needs no channel of its own.
