# `/status/{code}`

Ask for a status of your choosing, to test how your payer handles a paid packet whose answer is an
error:

```sh
toon send g.toon.echo --amount 10 --path /status/503 --seal-to "$ECHO_ILP_URL" --yes --json
```

The answer carries status 503 and a Mirror. **An error answer is still paid:** the connector
fulfils any HTTP answer, and a packet's payment is taken up front. A `code` that is not a status
the stack can send with a Mirror (non-numeric, outside 200 to 599, or `204` and `304`, whose
bodies HTTP drops) answers 400 with a Mirror. Any other path is mirrored with status 200.
