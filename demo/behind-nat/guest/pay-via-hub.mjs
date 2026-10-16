// Runs on the Linode payer: pay an app behind NAT through the devnet connector.
//
//   node pay-via-hub.mjs <connector URL> <destination> <seal key> <message>
//
// The payer opens a channel to the devnet connector at its public https URL
// and pays it. The devnet connector forwards the packet over the Anyone overlay
// to the agent node behind NAT. The request is sealed to that node's seal key,
// so the devnet connector carries it and cannot read it. The payer needs no
// anon daemon of its own.
//
// Its wallet is a mnemonic made here, kept in a 0600 file beside this script,
// and never printed.
import { ToonClient, generateMnemonic } from '@toon-protocol/client';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';

const [connector, destination, sealTo, message] = process.argv.slice(2);
const mnemonicFile = new URL('./hub-payer.mnemonic', import.meta.url);
if (!existsSync(mnemonicFile)) writeFileSync(mnemonicFile, generateMnemonic(), { mode: 0o600 });

const client = await ToonClient.create({
  connector,
  chain: 'evm',
  mnemonic: readFileSync(mnemonicFile, 'utf8').trim(),
  channelStore: new URL('./hub-channels.json', import.meta.url).pathname,
});
try {
  await client.wallet.faucet('evm');
  await new Promise((r) => setTimeout(r, 8000));
} catch {
  // Already funded inside the faucet's cooldown.
}
const open = await client.channel.open({ deposit: 100000n });
console.log(`channel to the connector  ${open.channel.channelId}  deposit ${open.depositTotal}`);

const answer = await client.send(destination, { body: { hello: message } }, { sealTo });
if (answer.fulfilled) {
  console.log(`FULFILL ${answer.status}  paid ${answer.claim.amount} to the connector`);
  console.log(answer.text());
} else {
  console.log(`REJECT ${answer.code} by ${answer.refusedBy}: ${answer.message}`);
  process.exitCode = 3;
}
await client.close();
