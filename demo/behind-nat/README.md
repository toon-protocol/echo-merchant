# An app behind NAT, paid from another network

**Live: Base and Solana mainnet · This demo: devnet, test USDC**

An Echo Merchant runs in a VM behind NAT. No port is forwarded and nothing listens for the
outside. A Linode on the public internet pays it through the devnet relay, and the relay
carries the packet to the VM over the Anyone overlay. The VM's node lands the payment on
Base Sepolia.

[`demo.sh`](demo.sh) builds both machines from nothing, runs the payments, shows the
evidence and deletes both machines. [`recording/behind-nat.cast`](recording/behind-nat.cast)
is one full run.

```sh
asciinema play recording/behind-nat.cast
```

## The path

```
 the public internet                         this machine, behind a home router and the ISP's NAT
 ┌───────────────┐  https   ┌─────────────────┐            ┌─────────────────────────────────────┐
 │ Linode Nanode │ ───────► │ devnet relay    │  Anyone    │ QEMU VM, user-mode NAT, no hostfwd  │
 │ toon-client   │  pays 3  │ the hub         │  overlay   │   toon agent node                   │
 │               │          │ anon, SOCKS out │ ─────────► │     anon: outbound circuits only    │
 └───────────────┘          └─────────────────┘  pays 2    │     connector on 127.0.0.1          │
                                                           │     echo-merchant, no published port│
                                                           └─────────────────────────────────────┘
```

- **The provider** is a Debian 13 VM under QEMU with user-mode networking. That is a NAT: the
  VM can open connections out, and nothing outside can open one in. The demo forwards no
  port, not even SSH. It drives the VM through the QEMU guest agent on a unix socket.
- **The hub** is the devnet relay, `proxy.relay.devnet.toonprotocol.dev`. `toon join devnet`
  makes the provider's peering toward it. The relay learns no routes, so its operator peers
  back toward the provider's `.anyone` address and routes the app's address
  ([toon-protocol/relay#268](https://github.com/toon-protocol/relay/pull/268)). The relay
  dials out over the overlay; the provider never accepts a connection from the internet.
- **The payer** is a Linode Nanode, on a different network. It runs
  [`toon-client`](https://github.com/toon-protocol/toon-client) and nothing else of TOON. It
  pays the relay at its public https address and seals the request to the provider's key, so
  the relay carries it and cannot read it. It runs no anon daemon for this.

The demo also pays the provider directly over the overlay, and has the provider pay out
through the hub, so the recording shows all three paths.

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
export RELAY_SSH=root@<the devnet relay's host>   # only the relay's operator has this
./demo.sh run                          # about 10 minutes
```

Without `GAS_FUNDER_MNEMONIC` the demo prints the provider's address and waits until you
fund it from a public Base Sepolia faucet.

The hub steps need the devnet relay's operator: peering the relay toward a new node is a
signed write on its operator surface. Without `RELAY_SSH` the demo skips `hub` and
`pay-hub` and runs the rest, the direct payment over the overlay included. To try the hub
path on a node of your own, ask in the TOON community for a peering, or run a relay with
[the same bundle](https://github.com/toon-protocol/relay/tree/main/deploy).

`run` is `up`, `provider`, `hub`, `pay-hub`, `pay`, `settle`, `hop` and `evidence` in order,
then `down`, which runs even when a step fails. Each step also runs on its own.

| Step | What it does |
| --- | --- |
| `up` | Boots the VM from the Debian cloud image and creates one `g6-nanode-1`, tagged `toon-demo` |
| `provider` | In the VM: installs `toon` v0.2.0, `toon init`, `toon wallet fund`, gas, `toon up`, `toon join devnet`, `toon add echo` |
| `hub` | On the relay, as its operator: `runtime-peerings.sh node`, a peering toward the provider's `.anyone` address and a route for its app |
| `pay-hub` | On the Linode: a channel to the relay, then one request to the app behind NAT, sealed to the provider ([`guest/pay-via-hub.mjs`](guest/pay-via-hub.mjs)) |
| `pay` | On the Linode: `toon-client describe`, `faucet`, `channel open`, `send`, straight to the provider over the overlay |
| `settle` | In the VM: `toon channel land` on each inbound channel, then each claim's transaction hash, read off Base Sepolia |
| `hop` | In the VM: pays `g.toon.relay.gas` through the devnet hub, hop by hop |
| `evidence` | `ss -tlnp` in the VM and on the host, `nmap` of the home's public address from the Linode |
| `down` | Removes the relay's peering and route toward the provider, deletes the Linode this demo made, stops and deletes the VM |

It costs a few cents of Linode time and 0.002 Base Sepolia ETH, plus the relay's gas for
one channel deposit. The USDC is devnet USDC from the faucet.

## What it prints

From the run in the recording, on 2026-10-15. The `.anyone` address, the routes and the
channels were new for that run. The machines and the relay's demo peering are gone; the
transactions are on Base Sepolia.

The provider gets gas, joins the devnet and adds the Echo Merchant at price 2. Then the
relay's operator peers back:

```text
host$ cast send --mnemonic "$GAS_FUNDER_MNEMONIC" --value 0.002ether 0x79eB3db2…1cBa0d
funding tx 0x75597b93ddb9f6aa98c452dae76083c2d148fc54082fbc71aa45df2d551cd4a4 (status 0x1)
provider-vm$ toon join devnet --deposit 1000000 --yes
Joined devnet: peered with https://proxy.relay.devnet.toonprotocol.dev/ilp and forwarding g.toon to it, deposit 1000000.
Its connector forwards back to you only if its operator creates a peering toward you in return.
relay-operator# ./runtime-peerings.sh node demo-0206ab32151f7edb http://ufnjnqsm…tbad.anyone/ilp g.toon.0206ab32151f7edb.echo 100000 3
{"id":"demo-0206ab32151f7edb","fee":1,"max_packet_amount":100000,"source":"runtime","channel":{"id":"0x86025dcf86b8866ed47690523e60ceb591c7ee3992533f6b5f6acb5a29d54509","status":"created","chain":"evm"}}
{"prefix":"g.toon.0206ab32151f7edb.echo","peer_id":"demo-0206ab32151f7edb","price":3,"source":"runtime"}
```

The payer, on the Linode, pays the relay at its public address. The relay forwards over the
overlay to the VM behind NAT:

```text
linode-payer# node pay-via-hub.mjs https://proxy.relay.devnet.toonprotocol.dev g.toon.0206ab32151f7edb.echo 0x040206ab…f9c9 'from a linode, through the devnet relay'
channel to the hub  0xba5f835de777f60ddff4738312b697cbe867cb99908bbb773b09747e6344229e  deposit 100000
FULFILL 200  paid 3 to the hub
{"payment":{"amount":"2","chain":"evm","payer":"evm:0x86025dcf…4509"},"request":{"body":"{\"hello\":\"from a linode, through the devnet relay\"}",…,"host":"172.18.0.2:3100"},"method":"POST","path":"/"}}
```

The payer paid the relay 3. The relay paid the provider 2 on its own channel,
`0x86025dcf…4509`, which the Mirror names as the payer. The Echo Merchant answered from
`172.18.0.2:3100`, the Docker network inside the VM.

The payer also pays the provider directly over the overlay, with toon-client's own `anon`
daemon:

```text
linode-payer# npx toon-client send g.toon.0206ab32151f7edb.echo --json-body --body '{"hello":"from a linode"}'
FULFILL 200  (http)
  paid        2 (0.000002 USDC)  channel 0xd232b9df43663eda438d9d296effd8d4e5831267ed400636c261d390101f6499
```

The provider lands both channels on chain:

```text
provider-vm$ toon channel land 0x86025dcf86b8866ed47690523e60ceb591c7ee3992533f6b5f6acb5a29d54509
Landed: 0x86025dcf…4509 evm inbound open (counterparty 0x3f43d923…f308, collateral 99998, landed 2, watermark 2)
provider-vm$ toon channel land 0xd232b9df43663eda438d9d296effd8d4e5831267ed400636c261d390101f6499
Landed: 0xd232b9df…6499 evm inbound open (counterparty 0x06b72e76…c7f1, collateral 99998, landed 2, watermark 2)
```

`0x3f43d923…f308` is the devnet relay's settlement address. The claims, from the provider's
settlement key:

- through the relay: [`0x4b7f7860e3485d438008c7abcc77286f80a18aba3645395adc66ff72b54ff21a`](https://sepolia.basescan.org/tx/0x4b7f7860e3485d438008c7abcc77286f80a18aba3645395adc66ff72b54ff21a)
- direct: [`0x9d7e7544fe497b95e872073ca51fb4f71366e444ac5d6e5a3c25ca8dd50e9180`](https://sepolia.basescan.org/tx/0x9d7e7544fe497b95e872073ca51fb4f71366e444ac5d6e5a3c25ca8dd50e9180)

`toon channel land` prints no transaction hash, so `settle` reads it from the
x402BatchSettlement contract's logs. In the recorded run the RPC had not indexed them yet
when it looked, so the recording shows `landed 2` and no hash. The script now looks again.
The hashes above are from the provider's transaction list on Base Sepolia.

From behind the same NAT, the provider pays out hop by hop, through the hub:

```text
provider-vm$ toon send g.toon.relay.gas --amount 1001 --seal-to https://proxy.gas.devnet.toonprotocol.dev/ilp --body ~/body.json --yes
Fulfilled: 1001 base units to g.toon.relay.gas. The app answered 422.
```

The gas station answered 422 because it wants a real gas request, not `{"hello":…}`. A 422
is still a paid answer.

Nothing listens for the outside:

```text
provider-vm# ss -tlnp      # every listener on 127.0.0.x; the connector on 127.0.0.1
host$ ss -tlnp | grep qemu || echo 'no listening socket'
no listening socket
linode-payer# nmap -n -Pn -T4 --top-ports 1000 <home-ip>
All 1000 scanned ports on <home-ip> are in ignored states.
Not shown: 1000 filtered tcp ports (no-response)
```

Teardown takes the relay's demo peering away again:

```text
relay-operator# ./runtime-peerings.sh remove demo-0206ab32151f7edb g.toon.0206ab32151f7edb.echo
the relay no longer peers toward demo-0206ab32151f7edb or routes g.toon.0206ab32151f7edb.echo. Its peers: gas echo store
```

## What it shows, and what it does not

- An app behind NAT answered requests from a different network, paid on two paths: through
  the devnet relay, and directly over the overlay. Both payments landed on Base Sepolia.
- Nothing listened for the outside. In the VM every listening socket is on `127.0.0.1`. On
  the host, QEMU holds no listening socket. From the Linode, every scanned port on the home's
  public address is filtered.
- The hub path needs the hub's operator. `join` makes a peering one way; the relay forwards
  to the node only after its operator peers back. That is a signed write today, not
  something a node can ask for.
- A hidden service hides where the app is reachable. It does not hide who pays: the channels
  are on a public chain.

## Keys and secrets

The provider's mnemonic and passphrase are made in the VM and stay there. The payer's are
made on the Linode and stay there. Both are deleted with their machine. The Linode root
password is random and never shown; login is by an SSH key made in `.state/`. The home's
public address is masked as `<home-ip>` in everything the demo prints. The Linode token is
passed to `linode-cli` in its environment and never printed. `GAS_FUNDER_MNEMONIC` is read
by `cast` from the environment, signs on Base Sepolia only, and is never printed. The relay
steps run its own `sign-write.sh` with its operator key on the relay; the recording shows
the commands and the answers, never a signature or a token.

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
- The devnet relay did not forward `g.toon.echo`: nobody had made its peering toward the
  Echo Merchant, so a joined node got `F02`. It is made now, at runtime, and recorded in
  the relay's `runtime-peerings.sh`
  ([toon-protocol/relay#268](https://github.com/toon-protocol/relay/pull/268)). The Echo
  flow's step 4 holds again.
- The relay could peer toward no hidden service: it had no `socks_proxy`. relay#268 adds an
  `anon` SOCKS client and the key.
- [toon-protocol/connector#1505](https://github.com/toon-protocol/connector/issues/1505):
  the connector resolves its SOCKS host once, at boot, and keeps a failed lookup until it
  restarts.
- [toon-protocol/relay#269](https://github.com/toon-protocol/relay/pull/269):
  `runtime-peerings.sh remove` stopped after its first delete. The demo sends both deletes
  itself until that merges.
- [toon-protocol/connector#1446](https://github.com/toon-protocol/connector/issues/1446):
  a packet rejected `F02` is still charged on the relay's connector build. It is fixed on
  the connector's `main` and not yet released.
