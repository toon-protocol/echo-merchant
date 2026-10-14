# Failures you must handle

The connector produces these, never the app. Each takes one command. No `toon` command sends
unpaid: `toon probe` answers with a reject that states the path's cost, not the greeting, so the
greeting is met with [toon-client](paying-with-toon-client.md).

| You see | One command |
| --- | --- |
| The **greeting**: the route's terms, instead of the work | `npx toon-client send g.toon.echo --amount 0 --body x`, a packet that carries no value: it exits `6`, the connector having "answered 402 with terms". A bare `curl -X POST "$ECHO_ILP_URL"` is not a packet, and is answered `400` |
| `F03`, an underpayment | `toon send g.toon.echo --amount 1 --body big.bin --seal-to "$ECHO_ILP_URL" --yes`, with `big.bin` a few KiB, so the price is above 1 |
| `F02`, no route | `toon send g.toon.nowhere --amount 10 --yes` |

A rejected `toon send` exits 1 and reports the code in `reject.code`. A reject that states a cost
reports `cost` and `complete`; send that as `--amount`.
