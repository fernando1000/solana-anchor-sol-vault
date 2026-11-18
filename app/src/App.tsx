import { useState } from "react";
import type { UiWalletAccount } from "@wallet-standard/react";
import { Vault } from "./Vault";
import { ConnectButtons } from "./wallet";

const short = (address: string) => `${address.slice(0, 4)}…${address.slice(-4)}`;

export default function App() {
  const [account, setAccount] = useState<UiWalletAccount | null>(null);

  return (
    <main className="app">
      <header>
        <div>
          <span className="eyebrow">Solana devnet</span>
          <h1>SOL Vault</h1>
        </div>
        {account ? (
          <span className="address">{short(account.address)}</span>
        ) : (
          <div className="connect">
            <ConnectButtons onConnect={setAccount} />
          </div>
        )}
      </header>
      {account ? (
        <Vault account={account} />
      ) : (
        <p className="hint">Connect a wallet to see your vault.</p>
      )}
    </main>
  );
}
