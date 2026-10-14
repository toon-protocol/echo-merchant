# Paying with toon-client

[`toon-client`](https://github.com/toon-protocol/toon-client) is the payer, as a Node.js
library and a command line, in the package `@toon-protocol/client`. It pays the Echo
Merchant's connector directly: no agent node, no peering, no route to add. It works the same
against a connector at a `.anyone` address and at a clearnet one.

## 1. Install it and make an identity

```sh
npm install @toon-protocol/client
export TOON_CONNECTOR=<the Echo Merchant's connector URL>
npx toon-client init                 # an encrypted keystore at ~/.toon/keystore.json
```

`TOON_CONNECTOR` is the connector's URL without `/ilp`: `http://<address>.anyone` for a
hidden service, `https://<hostname>` for a clearnet one. `init` prints the phrase once:
write it down.

## 2. See what it charges

Free, and it needs no keys:

```sh
npx toon-client describe
```

It lists the chains the connector is paid on and every route with its price. Behind an agent
node, the Echo Merchant's route is `g.toon.<segment>.echo`. The devnet box publishes
`g.toon.echo` as its own address, so `send` with no destination reaches it there.

## 3. Fund it and open a channel

On the devnet:

```sh
npx toon-client faucet                             # devnet USDC, no ETH needed
npx toon-client channel open --deposit 1000000     # 1 USDC, opened with no gas
```

On the sandbox there is no faucet and no facilitator. Fund the address `toon-client
identity` shows from the sandbox's throwaway keys, as toon_cli's [end-to-end
run](https://github.com/toon-protocol/toon_cli/blob/main/docs/end-to-end.md) does, and pay
the deposit's gas yourself:

```sh
export TOON_RPC_URL=http://localhost:8545 TOON_CHAIN=evm
npx toon-client channel open --deposit 1000000 --deposit-gas self
```

## 4. Pay and read the Mirror

```sh
npx toon-client send g.toon.<segment>.echo --json-body --body '{"hello":"echo"}'
```

```text
FULFILL 200  (http)
  paid        1  channel 0x11e3…b0d8
  cumulative  1

{"payment":{"amount":"1","chain":"evm","payer":"evm:0x11e3…b0d8"},"request":{"body":"{\"hello\":\"echo\"}","body_encoding":"utf8",…,"method":"POST","path":"/"}}
```

`toon-client` pays the route's price and no more, so `paid` and the Mirror's
`payment.amount` agree. Step 6 of [the Echo flow](echo-flow.md) explains every field of the
Mirror.

## 5. Ask for a status

`--target` is a path beneath the route, so it has no leading slash:

```sh
npx toon-client send g.toon.<segment>.echo --target status/503
```

The answer is `FULFILL 503` with a Mirror, paid like any other. `--target /status/503` is
refused `F00` before the app sees it. [`/status/{code}`](status-codes.md) lists the codes.

## 6. Meet the greeting

A packet that carries no value gets the route's terms instead of the work:

```sh
npx toon-client send g.toon.<segment>.echo --amount 0 --body x
```

It exits `6`, saying the connector "answered 402 with terms" for the route at its price.

## 7. Leave the channel

```sh
npx toon-client channel close        # costs gas; starts the withdrawal delay
npx toon-client channel settle       # once the delay has passed: the unspent deposit returns
```

`npx toon-client channel status` shows what you deposited, what you signed and what is left.

## From code

```ts
import { ToonClient } from '@toon-protocol/client';

const client = await ToonClient.create({
  connector: process.env.TOON_CONNECTOR,
  mnemonic: process.env.TOON_MNEMONIC,
  channelStore: `${process.env.HOME}/.toon/channels.json`,
});

// The first paid send opens the channel.
const answer = await client.send('g.toon.<segment>.echo', { body: { hello: 'echo' } });
if (answer.fulfilled) {
  console.log(answer.status, answer.json());     // the Mirror
} else {
  console.log(answer.code, answer.message);      // a refusal, not an exception
}

await client.close();
```

## A hidden service

Point `TOON_CONNECTOR` at the `.anyone` address and the command line starts a pinned `anon`
daemon for the run. Every byte then rides the overlay, chain RPC included, so a `localhost`
RPC cannot be reached. That matters on the sandbox. From code, give the library an `anon`
daemon's SOCKS port and keep the local RPC off the overlay:

```ts
const client = await ToonClient.create({
  connector: 'http://<address>.anyone',
  socksProxy: 'socks5h://127.0.0.1:<port>',
  rpcUrl: 'http://localhost:8545',
  proxyRpc: false,
  depositGas: 'self',
  chain: 'evm',
  mnemonic: process.env.TOON_MNEMONIC,
  channelStore: `${process.env.HOME}/.toon/channels.json`,
});
```

toon-client's [hidden-service
guide](https://github.com/toon-protocol/toon-client/blob/main/docs/hidden-service.md) says
more.