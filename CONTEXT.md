# Echo Merchant

A priced TOON route on devnet that anyone can pay, which answers each packet by showing the payer
what arrived and what it cost, so a payer can prove its whole payment flow before risking money.
Words borrowed from the connector (**route**, **price**, **charge**, **fee**, **cost**,
**greeting**, **payment channel**) keep the meanings in `connector/CONTEXT.md`.

## Language

**Echo Merchant**:
The app behind the priced route, together with that route. Its counterparty is a developer
building a payer; it is not a template for building merchants.
_Avoid_: test merchant, free route (a free route in TOON takes no money), faucet

**Mirror**:
The Echo Merchant's answer to a paid packet: the payload as it arrived, and the value as it
arrived, set beside the route's price and the charge that price gives for this packet.
_Avoid_: receipt, echo response

**Echo flow**:
The path a newcomer is told to walk: fund a wallet, open a channel, pay the Echo Merchant, read
the Mirror, then withdraw the channel's collateral and see the unspent deposit return.
_Avoid_: demo, tutorial, happy path

**Refund**:
Not part of the Echo Merchant. A paid packet's value is never sent back; the money a payer gets
back is the unspent deposit, returned when the channel's collateral is withdrawn.
