# The Echo flow

Start from a machine with nothing on it. Each step is one command.

**1. Install `toon` and create a wallet.** The passphrase seals the keystore; `toon` reads it
from the file `TOON_PASSPHRASE_FILE` names.

```sh
curl -fsSL https://raw.githubusercontent.com/toon-protocol/toon_cli/main/install.sh | sh
mkdir -p ~/.config/toon
printf '%s\n' 'a long passphrase you keep somewhere safe' > ~/.config/toon/passphrase
chmod 600 ~/.config/toon/passphrase
export TOON_PASSPHRASE_FILE=~/.config/toon/passphrase
toon init --network devnet --accept-anyone-terms     # add --solana to pay from Solana too
```

`init` prints the mnemonic once: write it down.
[Your first agent node](https://github.com/toon-protocol/toon_cli/blob/main/docs/guide/first-agent-node.md)
says more.

**2. Fund the wallet from the faucet.**

```sh
toon wallet fund
toon wallet balances
```

The faucet sends the mock USDC and no gas. Opening and withdrawing a channel are transactions, so
send a little Base Sepolia ETH to the address `toon wallet show` lists, from a public faucet. A
Solana channel needs a little devnet SOL instead.

**3. Start the agent node.**

```sh
toon up
toon status
```

**4. Open a channel.** Joining the devnet opens one, with a deposit, toward the network's hub, which
reaches `g.toon.echo`:

```sh
toon join devnet --deposit 1000000 --yes     # add --chain evm or --chain solana to choose
toon channel list                            # note the outbound channel's id
```

The deposit is your whole exposure. The Echo Merchant takes dust from it.

**5. Pay the Echo Merchant.** The packet is sealed to the Echo Merchant's connector, so name its
`/ilp` URL once. Every later command reads it from `ECHO_ILP_URL`.

```sh
export ECHO_ILP_URL=<the Echo Merchant connector's /ilp URL>
toon describe "$ECHO_ILP_URL"                # its route and price, for free
echo '{"hello":"echo"}' > body.json
toon send g.toon.echo --amount 10 --seal-to "$ECHO_ILP_URL" --body body.json --yes --json
```

`--amount` is the most you will pay. The hub charges to forward, so send more than the price:
`toon probe g.toon.echo --seal-to "$ECHO_ILP_URL"` reports the path's `cost` when you do not know it.

**6. Read the Mirror.** It is JSON with a stable shape:

```json
{
  "request": {
    "method": "POST",
    "path": "/",
    "headers": { "content-type": "application/json" },
    "body": "{\"hello\":\"echo\"}\n",
    "body_encoding": "utf8"
  },
  "payment": { "payer": "evm:0x…", "amount": "1", "chain": "…" }
}
```

- `request` is what arrived: method, path with query, headers, body. A body that is valid UTF-8
  comes back as text (`"utf8"`); any other body comes back as base64 (`"base64"`), so you can compare
  it byte for byte.
- `payment` is what the connector attributed the packet to: your channel (`payer`), the **charge**
  (`amount`, a string) and the `chain`. It is `null` when the request carried no attribution. The
  Mirror never invents a payment.
- The Mirror does not carry the price. Read it from `toon describe`.

**The charge is not what you signed for.** `payment.amount` is the **charge**, what this route's
price comes to for this packet: 1, plus 1 per KiB. The **cost** is what you signed for, the
`--amount` the packet was paid with. The cost minus the charge is the **fees** the hops between you
and the Echo Merchant took, plus whatever you sent above the price, which the Echo Merchant's
connector keeps. `toon send` reports the cost as `paid`; compare it with
`payment.amount` to see where your value went. Send a larger body and watch the charge grow.

**7. Withdraw the channel's collateral.** Take the channel id from `toon channel list`:

```sh
toon channel withdraw <id>
toon channel list
```

The unspent deposit returns to your wallet. See [Withdrawing](#withdrawing) first.

## Withdrawing

`toon channel withdraw` starts or finishes a withdrawal, "the chain decides which". **On EVM
withdrawing is two steps with a delay between them**: the first run starts it, and after the
chain's delay you run the same command again to finish it. Until then the deposit has not
returned; the first run did not fail. `toon channel list` shows the channel's status.
