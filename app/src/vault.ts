import {
  appendTransactionMessageInstruction,
  compileTransaction,
  containsBytes,
  createSolanaRpc,
  createTransactionMessage,
  getBase58Decoder,
  getBase64EncodedWireTransaction,
  getBase64Encoder,
  pipe,
  setTransactionMessageFeePayerSigner,
  setTransactionMessageLifetimeUsingBlockhash,
  signAndSendTransactionMessageWithSigners,
  type Address,
  type Instruction,
  type Signature,
  type TransactionSendingSigner,
} from "@solana/kit";
import {
  DEPOSITED_EVENT_DISCRIMINATOR,
  fetchMaybeVaultState,
  findStatePda,
  findVaultPda,
  getDepositInstructionAsync,
  getInitializeInstructionAsync,
  getWithdrawInstructionAsync,
  parseDepositedEvent,
  parseWithdrawnEvent,
  WITHDRAWN_EVENT_DISCRIMINATOR,
  type DepositedEvent,
  type WithdrawnEvent,
} from "./generated";

export const RPC_URL = import.meta.env.VITE_RPC_URL ?? "https://api.devnet.solana.com";
export const rpc = createSolanaRpc(RPC_URL);

// Builds a transaction with one instruction and simulates it first: if the program
// would reject it, the wallet never opens. Then the wallet signs and sends it,
// and we wait until the network confirms it.
async function send(signer: TransactionSendingSigner, instruction: Instruction) {
  const { value: blockhash } = await rpc.getLatestBlockhash().send();
  const message = pipe(
    createTransactionMessage({ version: 0 }),
    (m) => setTransactionMessageFeePayerSigner(signer, m),
    (m) => setTransactionMessageLifetimeUsingBlockhash(blockhash, m),
    (m) => appendTransactionMessageInstruction(instruction, m),
  );
  const unsigned = getBase64EncodedWireTransaction(compileTransaction(message));
  const { value: simulation } = await rpc
    .simulateTransaction(unsigned, { encoding: "base64", sigVerify: false })
    .send();
  if (simulation.err) throw new Error(errorMessage(simulation.logs));
  const signature = getBase58Decoder().decode(
    await signAndSendTransactionMessageWithSigners(message),
  ) as Signature;
  for (let i = 0; i < 60; i++) {
    const { value } = await rpc.getSignatureStatuses([signature]).send();
    if (value[0]?.err) throw new Error(JSON.stringify(value[0].err));
    if (value[0]?.confirmationStatus === "confirmed") return signature;
    if (value[0]?.confirmationStatus === "finalized") return signature;
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`Transaction not confirmed: ${signature}`);
}

export async function fetchVault(owner: Address) {
  const [state] = await findStatePda({ owner });
  const [vault] = await findVaultPda({ state });
  const account = await fetchMaybeVaultState(rpc, state);
  const { value: balance } = await rpc.getBalance(vault).send();
  return { unlockAt: account.exists ? account.data.unlockAt : null, balance };
}

export async function createVault(signer: TransactionSendingSigner, lockSeconds: number) {
  const unlockAt = BigInt(Math.floor(Date.now() / 1000) + lockSeconds);
  return send(signer, await getInitializeInstructionAsync({ owner: signer, unlockAt }));
}

export async function deposit(signer: TransactionSendingSigner, sol: number) {
  const amount = BigInt(Math.round(sol * 1e9));
  return send(signer, await getDepositInstructionAsync({ owner: signer, amount }));
}

export async function withdraw(signer: TransactionSendingSigner) {
  return send(signer, await getWithdrawInstructionAsync({ owner: signer }));
}

export type VaultEvent =
  ({ name: "Deposited" } & DepositedEvent) | ({ name: "Withdrawn" } & WithdrawnEvent);

// Anchor writes each event as "Program data: <base64>" in the transaction logs.
export async function fetchEvents(signature: Signature) {
  const transaction = await rpc
    .getTransaction(signature, {
      commitment: "confirmed",
      encoding: "json",
      maxSupportedTransactionVersion: 0,
    })
    .send();
  const events: VaultEvent[] = [];
  for (const line of transaction?.meta?.logMessages ?? []) {
    if (!line.startsWith("Program data: ")) continue;
    const data = getBase64Encoder().encode(line.slice("Program data: ".length));
    if (containsBytes(data, DEPOSITED_EVENT_DISCRIMINATOR, 0)) {
      events.push({ name: "Deposited", ...parseDepositedEvent(data) });
    } else if (containsBytes(data, WITHDRAWN_EVENT_DISCRIMINATOR, 0)) {
      events.push({ name: "Withdrawn", ...parseWithdrawnEvent(data) });
    }
  }
  return events;
}

// When our program rejects a transaction, Anchor logs "Error Message: Vault is still locked."
function errorMessage(logs: readonly string[] | null) {
  const line = logs?.find((log) => log.includes("Error Message: "));
  return line ? line.split("Error Message: ")[1].replace(/\.$/, "") : "Transaction failed";
}
