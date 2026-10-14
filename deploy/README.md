# Running an Echo Merchant box

This directory is the whole deployment of the **Echo Merchant**: the payment proxy
(a TOON connector) and the app behind it. It is a copy of gas-station's `deploy/`
bundle with the gas-station routes, names and ports replaced. `./bootstrap.sh` on a
fresh Ubuntu host is the entire install.

```
   payer ──── :443 ────▶ nginx ──▶ connector :4000 ──▶ echo-merchant :3000
                                   meters & settles     answers with a Mirror
```

The connector serves one route, `g.toon.echo`, priced `{ base = 1, per_kib = 1 }` in
USDC base units. The price must stay above zero: in TOON a route with price 0 takes
no money (ADR 0001). It accepts channels on Base Sepolia, and on Solana devnet once
`SETTLEMENT_SOLANA=on`.

## Files

| File | What it is |
|---|---|
| `docker-compose.yml` | The containers. The only file that names an image tag: the connector is pinned to one release, the Echo Merchant follows `release`. |
| `connector.toml.template` | The connector's config: the route, its price, how it settles. Rendered to `connector.toml`; holds paths, no secret. |
| `nginx/node.conf.template` | The TLS edge. Rendered to `nginx/conf.d/node.conf`. |
| `render.sh` | Fills the templates from `.env` and writes the operator credential files. |
| `bootstrap.sh` | Fresh-host install: firewall, swap, docker, keys, render, start, TLS. |
| `init-letsencrypt.sh` | Issues or reuses the certificate. |
| `auto-apply.sh`, `toon-auto-apply-echo.{service,timer}` | Every 5 minutes, fast-forward to `main`, render, pull, `up -d`, and restart the connector if its inputs changed. |
| `docker-compose.shared-edge.yml` | The shared-edge overlay (off by default). |
| `.env.example` | Every variable. |

## What the operator does, once

1. Point an A-record for `proxy.echo.<DOMAIN>` at the box.
2. Clone this repo to `/root/echo-merchant`, copy `deploy/.env.example` to `deploy/.env`
   and fill it in (`OPERATOR_BEARER_TOKEN`, `OPERATOR_WRITE_KEY`, `DOMAIN`,
   `LETSENCRYPT_EMAIL`).
3. Run `deploy/bootstrap.sh` as root. It generates `signer.key`, `settlement.key` and
   `settlement-solana.key`, renders the config and starts the stack.
4. Back up `settlement.key` and `settlement-solana.key`: payers open channels toward them.
5. Fund `settlement-solana.key` with devnet SOL, set `SETTLEMENT_SOLANA=on` in `.env`,
   and let the auto-apply timer (or `./render.sh && docker compose up -d`) load it.
   The connector will not boot with an unfunded Solana key, which is why this is a
   separate step.
6. Install the timer: copy `toon-auto-apply-echo.service` and `.timer` to
   `/etc/systemd/system/`, then `systemctl enable --now toon-auto-apply-echo.timer`.
   After that a merge to `main` reaches the box on its own.

## Running behind the shared edge

On the shared devnet host one Caddy edge owns ports 80 and 443. Uncomment
`COMPOSE_FILE=docker-compose.yml:docker-compose.shared-edge.yml` in `.env`. The overlay
disables nginx, certbot and watchtower, joins the connector to the `edge-echo` network
as `echo-proxy`, and moves its loopback publish to `127.0.0.1:4005`.

The host-side wiring is not done here. Choosing the next free loopback admin port,
creating the `edge-echo` network, and adding `proxy.echo.<DOMAIN>` to the edge's
routes is a change in `toon-protocol/infra`, filed there. The edge must keep what
nginx does here: the rate limit and the CORS header on `/ilp/identity`. Until that
ticket lands, port 4005 and the `edge-echo` name are this bundle's proposal.

## Bumping the connector

Change the image tag in `docker-compose.yml` and the matching literal in
`tests/deploy_bundle.rs` together, landing any `connector.toml.template` change first.
