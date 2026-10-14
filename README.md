<div align="center">

<img src="docs/assets/toon-icon.svg" alt="TOON icon" width="160">

# Echo Merchant

**Run a route that anyone can pay, and that shows each payer exactly what arrived.**

*It sells nothing, on purpose.* Payers pay it to prove their payment flow works. It answers
each paid packet with a Mirror: the request as it arrived, and the payment as it arrived.

[![CI](https://img.shields.io/github/actions/workflow/status/toon-protocol/echo-merchant/ci.yml?branch=main&style=flat-square&label=ci)](https://github.com/toon-protocol/echo-merchant/actions/workflows/ci.yml)
[![Image](https://img.shields.io/github/actions/workflow/status/toon-protocol/echo-merchant/publish-image.yml?branch=main&style=flat-square&label=image&color=c9a24a)](https://github.com/toon-protocol/echo-merchant/actions/workflows/publish-image.yml)
[![Reach](https://img.shields.io/badge/reach-hidden%20service%20%7C%20clearnet-lightgrey?style=flat-square)](docs/guide/running.md)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-b7410e?style=flat-square)](https://www.rust-lang.org)

[**Run it**](docs/guide/running.md) · [**Deploy a box**](deploy/README.md) · [**Pay it**](docs/guide/echo-flow.md) · [**Failures**](docs/guide/failures.md) · [**Guides**](#guides) · [**Development**](docs/development.md)

</div>

---

## Install

```sh
curl -fsSL https://toon.ar.io/install.sh | sh
```

## Three steps

**1. Create the agent node.** It is a hidden service unless you name a hostname.

```sh
toon init --network devnet --accept-anyone-terms     # or --clearnet <hostname>
```

**2. Fund it and start it.**

```sh
toon wallet fund                                     # mock USDC from the faucet
toon up
```

**3. Put the Echo Merchant behind its connector.** Payers can then pay it.

```sh
toon add echo --to relay --image ghcr.io/toon-protocol/echo-merchant:release \
  --env ECHO_MERCHANT_ADDR=0.0.0.0:3100 --price 1 --yes
```

This skips the passphrase file and the gas it settles with.
[Running the Echo Merchant](docs/guide/running.md) has every step, and how a payer reaches
it. The devnet's own, at `g.toon.echo`, runs from [`deploy/`](deploy/README.md).

## What holds

- **It holds nothing of yours.** The app is one axum router with no keys and no state, in
  an image with no shell.
- **It never invents a payment.** `payment` is what the connector attributed the packet
  to, and `null` when the request carried none.
- **Hidden by default.** As a hidden service it is reached at a `.anyone` address, and
  has no public hostname unless you give it one.
- **Test money.** On the devnet, payers pay in mock USDC from the faucet. **MIT licence.**

Below: why it is built this way, how the pieces fit, the [guides](#guides).

## Why it exists

Every other priced route on devnet sells something real, so when a payment fails a payer
cannot tell a broken payer from a broken service. The Echo Merchant sells nothing.

- The **Echo Merchant** is a priced route that anyone can pay, and that answers by showing
  the payer what it received: a **Mirror**, the request as it arrived and the payment as it
  arrived. You run it behind a connector, as a hidden service or at a clearnet hostname.
- A **payer** is the code someone is building. It pays a packet and reads the Mirror.

The Mirror is built so a payer can check its whole payment flow against it:

- **The Mirror never invents a payment.** `payment` is your channel, the charge and the
  chain, or `null` when the request carried no attribution.
- **The Mirror loses no byte.** A body that is valid UTF-8 comes back as text; any other
  body comes back as base64, so you can compare it byte for byte.
- **The Mirror shows the charge, not the cost.** Compare `payment.amount` with what
  `toon send` reports as `paid`: the gap is the hops' fees, plus what you sent above the
  price, which the Echo Merchant's connector keeps.
- **The Mirror comes back with any status you ask for.** `/status/503` answers 503 and a
  Mirror, and an error answer is still paid ([status codes](docs/guide/status-codes.md)).

### Why there is no refund

PayAI's echo merchant charges and then refunds; this one does not. A packet's payment is
taken up front and kept even on a reject, so a refund would be a second payment the other
way, and nothing in an app can send one. The money you get back is your unspent deposit,
which step 7 of [the Echo flow](docs/guide/echo-flow.md) returns. See
[ADR 0001](docs/adr/0001-mirror-not-refund.md).

## How it fits together

```
 an Echo Merchant: one connector, one app, one route
 ┌──────────────────────────────────────────────────────────────────┐
 │  .anyone or https ──► connector ────────► echo-merchant          │
 │                       meters & settles    answers with a Mirror  │
 └──────▲───────────────────────────────────────────────────┬───────┘
        │  a paid packet                           a Mirror │
        └──────── a payer ◄─────────────────────────────────┘
```

The **connector** meters and settles; the **Echo Merchant** behind it only answers. Behind
an agent node, the connector is `toon`'s, reached at its `.anyone` address or through your
own reverse proxy. On the devnet box, nginx passes `:443` to the connector on `:4000` and
the app listens on `:3000`. A payer seals each packet to the connector's `/ilp` URL.
[`CONTEXT.md`](CONTEXT.md) is the glossary.

### The devnet's route, for reference

| Property | What it is |
| --- | --- |
| Address | `g.toon.echo` |
| Price | 1 base unit, plus 1 per KiB of payload |
| Chains | Base Sepolia, Solana devnet |
| Network | **Devnet only.** The money is mock USDC from the faucet. |

Amounts are in the token's base units; on devnet the token has 6 decimals.

## Guides

| Guide | What it walks through |
| --- | --- |
| [Running the Echo Merchant](docs/guide/running.md) | An agent node as a hidden service or at a clearnet hostname, `toon add` with the image, the price, a payer peering with it, the sandbox |
| [Deploying a box](deploy/README.md) | The devnet's Echo Merchant: a connector and the app behind nginx, from `bootstrap.sh` |
| [The Echo flow](docs/guide/echo-flow.md) | Install, fund, open a channel, pay, read the Mirror, the charge against the cost, and withdrawing |
| [`/status/{code}`](docs/guide/status-codes.md) | Asking for an error status, what is still paid, and which codes answer 400 |
| [Failures you must handle](docs/guide/failures.md) | The greeting, an underpayment (`F03`), no route (`F02`), and reading a reject |

## Reference

- [`CONTEXT.md`](CONTEXT.md): the glossary, Mirror, Echo flow and refund.
- [`docs/adr/`](docs/adr/): the decisions behind the design, and why.
- [`docs/development.md`](docs/development.md): the checks, and how the image is published.
