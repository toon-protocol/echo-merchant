# Failures you must handle

The connector produces these, never the app. Each but the greeting takes one command. No `toon`
command sends unpaid: `toon probe` answers with a reject that states the path's cost, not the
greeting.

| You see | One command |
| --- | --- |
| The **greeting**: the route's terms, instead of the work | None: your own payer meets it when it sends a packet that carries no voucher. A bare `curl -X POST "$ECHO_ILP_URL"` is not a packet, and is answered `400`. `toon describe "$ECHO_ILP_URL"` prints the same terms |
| `F03`, an underpayment | `toon send g.toon.echo --amount 1 --body big.bin --seal-to "$ECHO_ILP_URL" --yes`, with `big.bin` a few KiB, so the price is above 1 |
| `F02`, no route | `toon send g.toon.nowhere --amount 10 --yes` |

A rejected `toon send` exits 1 and reports the code in `reject.code`. A reject that states a cost
reports `cost` and `complete`; send that as `--amount`.
