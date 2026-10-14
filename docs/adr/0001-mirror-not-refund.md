# The Echo Merchant mirrors a packet; it does not refund it

PayAI's echo merchant charges and then refunds, and a reader will expect this one to copy it. It
does not. It answers each paid packet with a **Mirror** (what arrived, and what it was charged),
and the money a newcomer gets back is the unspent deposit from `toon channel withdraw`. Three
facts of TOON decide this: a packet's payment is taken up front and kept even on a reject
(connector ADR 0042), so a refund must be a separate payment the other way; the connector's
client payout has no production caller, so an app cannot send one, and the only route left is a
raw on-chain transfer that teaches a model TOON does not have; and on devnet the money is mock
USDC from a faucet, so there is nothing at risk to return. A payer's real exposure in TOON is
its channel deposit, not a dust-priced packet, and withdrawing that deposit is the protocol's
own way of giving money back.

## Considered Options

- **On-chain refund from an app hot wallet** to the payer: buildable in a day, but the charge
  leaves a channel and the refund lands in a wallet, and the app would need keys and gas.
- **Client payout from the merchant's connector**: the TOON-native refund, but it needs connector
  work first (a caller that decides when to pay out, and a payout channel per client).

## Consequences

If a mainnet echo is ever wanted with "money comes back", the path is to give client payouts a
caller in the connector and make the Echo Merchant its first user, not to add a hot wallet here.
