<div align="center">

<img src="docs/assets/toon-icon.svg" alt="TOON icon" width="160">

# Echo Merchant

**Prove your payer works before you risk money on someone else's product.**

*It sells nothing, on purpose.* It answers each paid packet with a Mirror: the request as
it arrived, and the payment as it arrived. Devnet only, so the money is mock USDC.

[![CI](https://img.shields.io/github/actions/workflow/status/toon-protocol/echo-merchant/ci.yml?branch=main&style=flat-square&label=ci)](https://github.com/toon-protocol/echo-merchant/actions/workflows/ci.yml)
[![Image](https://img.shields.io/github/actions/workflow/status/toon-protocol/echo-merchant/publish-image.yml?branch=main&style=flat-square&label=image&color=c9a24a)](https://github.com/toon-protocol/echo-merchant/actions/workflows/publish-image.yml)
[![Network](https://img.shields.io/badge/network-devnet%20only-lightgrey?style=flat-square)](docs/guide/echo-flow.md)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-b7410e?style=flat-square)](https://www.rust-lang.org)

[**The Echo flow**](docs/guide/echo-flow.md) · [**Status codes**](docs/guide/status-codes.md) · [**Failures**](docs/guide/failures.md) · [**Guides**](#guides) · [**Development**](docs/development.md)

```sh
curl -fsSL https://raw.githubusercontent.com/toon-protocol/toon_cli/main/install.sh | sh
```

</div>

---

## Three steps

**1. Install `toon` and create a wallet** on the devnet.

```sh
curl -fsSL https://raw.githubusercontent.com/toon-protocol/toon_cli/main/install.sh | sh
toon init --network devnet --accept-anyone-terms     # add --solana to pay from Solana too
```

**2. Fund it and open a channel.** The deposit is your whole exposure.

```sh
toon wallet fund                                     # mock USDC from the faucet
toon up
toon join devnet --deposit 1000000 --yes             # a channel toward the network's hub
```

**3. Pay the Echo Merchant and read the Mirror.**

```sh
toon send g.toon.echo --amount 10 --seal-to "$ECHO_ILP_URL" --body body.json --yes --json
```

This skips the passphrase file, the gas a channel needs, and `ECHO_ILP_URL`.
[The Echo flow](docs/guide/echo-flow.md) has all seven steps, and ends by withdrawing the
deposit.

## What holds

- **It takes dust.** The price is 1 base unit, plus 1 per KiB of payload. The unspent
  deposit returns to your wallet when you withdraw.
- **It never invents a payment.** `payment` is what the connector attributed the packet
  to, and `null` when the request carried none.
- **It holds nothing of yours.** The app is one axum router with no keys and no state.
- **Devnet only.** Base Sepolia or Solana devnet, paid in mock USDC from the faucet.

Below: why it is built this way, how the pieces fit, the [guides](#guides).

## Why it exists

Every other priced route on devnet sells something real, so when a payment fails you cannot
tell a broken payer from a broken service. The Echo Merchant sells nothing.

- A **payer** is the code you are building. It pays a packet and reads the answer.
- The **Echo Merchant** is a priced route on the TOON devnet that anyone can pay, and that
  answers by showing you what it received. At `g.toon.echo` it answers each paid packet
  with a **Mirror**: the request as it arrived, and the payment as it arrived.

The Mirror is built so a payer can check its whole payment flow against it:

- **The Mirror never invents a payment.** `payment` is your channel, the charge and the
  chain, or `null` when the request carried no attribution.
- **The Mirror loses no byte.** A body that is valid UTF-8 comes back as text; any other
  body comes back as base64, so you can compare it byte for byte.
- **The Mirror shows the charge, not the cost.** Compare `payment.amount` with what
  `toon send` reports as `paid` to see the fees the hops between you and the Echo
  Merchant took.
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
 the Echo Merchant box: one route, g.toon.echo
 ┌──────────────────────────────────────────────────────────────┐
 │  nginx :443 ──► connector :4000 ──► echo-merchant :3000      │
 │                 meters & settles    answers with a Mirror    │
 └──────▲───────────────────────────────────────────────┬───────┘
        │  a paid packet, via the devnet hub   a Mirror │
        └──────── your payer ◄──────────────────────────┘
```

The **connector** meters and settles; the **Echo Merchant** behind it only answers. Your
payer reaches it through a channel toward the network's hub, and seals each packet to the
connector's `/ilp` URL. [`CONTEXT.md`](CONTEXT.md) is the glossary.

### The route, for reference

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
| [The Echo flow](docs/guide/echo-flow.md) | Install, fund, open a channel, pay, read the Mirror, the charge against the cost, and withdrawing |
| [`/status/{code}`](docs/guide/status-codes.md) | Asking for an error status, what is still paid, and which codes answer 400 |
| [Failures you must handle](docs/guide/failures.md) | The greeting, an underpayment (`F03`), no route (`F02`), and reading a reject |

## Reference

- [`CONTEXT.md`](CONTEXT.md): the glossary, Mirror, Echo flow and refund.
- [`docs/adr/`](docs/adr/): the decisions behind the design, and why.
- [`deploy/README.md`](deploy/README.md): running an Echo Merchant box, connector and app.
- [`docs/development.md`](docs/development.md): the checks, and how the image is published.
