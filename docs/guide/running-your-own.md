# Running your own Echo Merchant

The Echo Merchant at `g.toon.echo` on the devnet runs from
[`deploy/`](../../deploy/README.md). This page runs a copy of your own behind an agent node,
with `toon`: as a **hidden service**, the default, or at a **clearnet** hostname. Use it to
test a payer on the sandbox, or against a merchant only you can reach.

It needs `toon` 0.2.0 or later, for `toon add --env`, and Docker, which runs the image.

## 1. Create the agent node

As a hidden service. The connector is reached at its own `.anyone` address, and nothing
needs a public hostname:

```sh
export TOON_PASSPHRASE_FILE=~/.config/toon/passphrase
toon init --network devnet --accept-anyone-terms --json > init.json
jq -r '.toon_apps[0].onion_endpoint' init.json      # its /ilp URL is http://<this>/ilp
```

At a clearnet hostname. The certificate and the reverse proxy that pass
`https://echo.example.com` to the listen address are yours to provide:

```sh
toon init --network devnet --clearnet echo.example.com --listen 127.0.0.1:8080 --json > init.json
```

`init` prints the mnemonic once. Toon_cli's [networks and
reachability](https://github.com/toon-protocol/toon_cli/blob/main/docs/guide/networks-and-reachability.md)
guide says more about both.

## 2. Fund it and start it

```sh
toon wallet fund
toon up
toon status
```

The merchant is paid, so it needs no deposit. It needs a little gas to settle what it is
paid: send some Base Sepolia ETH to the address `toon wallet show` lists.

## 3. Put the Echo Merchant behind the connector

`toon` runs an app's image with the app listening on port 3100, so move the Echo Merchant
there:

```sh
toon add echo --to relay --image ghcr.io/toon-protocol/echo-merchant:release \
  --env ECHO_MERCHANT_ADDR=0.0.0.0:3100 --price 1 --yes --json
```

It answers with the app's address, `g.toon.<segment>.echo`. Leave `--address` out: on the
devnet, `g.toon.echo` is the network's own Echo Merchant.

`--price` is what one packet costs, whatever its size. `toon` cannot set a price per KiB, so
your copy does not charge more for a larger body the way the devnet's does. To see an `F03`,
add it with `--price 2` and send `--amount 1`.

## 4. Pay it

A payer peers with your connector directly, then sends to the app's address:

```sh
export ECHO_ILP_URL=<your connector's /ilp URL>
toon describe "$ECHO_ILP_URL"
toon peer add "$ECHO_ILP_URL" --deposit 1000000 --id echo --yes
toon route add g.toon.<segment>.echo --peer echo
toon send g.toon.<segment>.echo --amount 10 --seal-to "$ECHO_ILP_URL" --body body.json --yes --json
```

A payer that pays a hidden service must be able to reach `.anyone` addresses: an agent node
made with `toon init` as a hidden service can. Then read the Mirror and withdraw as in [the
Echo flow](echo-flow.md), from step 6.

## On the sandbox

Give `init` `--network sandbox` instead. The `infra` sandbox's anvil is the chain, and it
has no faucet: fund both wallets from the sandbox's throwaway keys, as toon_cli's
[end-to-end run](https://github.com/toon-protocol/toon_cli/blob/main/docs/end-to-end.md)
does. The merchant and the payer need nothing else from the sandbox: no hub and no relay.