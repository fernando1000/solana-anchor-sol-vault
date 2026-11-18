import { useCallback, useEffect, useState } from "react";
import type { Address, Signature } from "@solana/kit";
import { useWalletAccountTransactionSendingSigner } from "@solana/react";
import type { UiWalletAccount } from "@wallet-standard/react";
import { createVault, deposit, fetchEvents, fetchVault, withdraw, type VaultEvent } from "./vault";

type VaultInfo = Awaited<ReturnType<typeof fetchVault>>;

const LOCK_OPTIONS = [
  { label: "2 minutes", seconds: 2 * 60 },
  { label: "1 hour", seconds: 60 * 60 },
  { label: "1 day", seconds: 24 * 60 * 60 },
];

const sol = (lamports: bigint) => (Number(lamports) / 1e9).toFixed(4);

function countdown(seconds: number) {
  if (seconds <= 0) return "Unlocked";
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
}

function describe(event: VaultEvent) {
  if (event.name === "Deposited") {
    return `Deposited ${sol(event.amount)} SOL · vault holds ${sol(event.total)} SOL`;
  }
  return `Withdrew ${sol(event.amount)} SOL · vault closed`;
}

export function Vault({ account }: { account: UiWalletAccount }) {
  const signer = useWalletAccountTransactionSendingSigner(account, "solana:devnet");
  const owner = account.address as Address;

  const [vault, setVault] = useState<VaultInfo | null>(null);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const [lockSeconds, setLockSeconds] = useState(LOCK_OPTIONS[0].seconds);
  const [amount, setAmount] = useState("0.5");
  const [activity, setActivity] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => setVault(await fetchVault(owner)), [owner]);

  useEffect(() => {
    refresh().catch((e) => setError(String(e)));
  }, [refresh]);

  useEffect(() => {
    const timer = setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000);
    return () => clearInterval(timer);
  }, []);

  async function run(action: () => Promise<Signature>) {
    setBusy(true);
    setError(null);
    try {
      const signature = await action();
      const events = await fetchEvents(signature);
      setActivity((items) => [...events.map(describe), ...items]);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!vault) return <p className="hint">Loading…</p>;

  return (
    <section className="card">
      {vault.unlockAt === null ? (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            run(() => createVault(signer, lockSeconds));
          }}
        >
          <select
            value={lockSeconds}
            onChange={(e) => setLockSeconds(Number(e.target.value))}
            disabled={busy}
          >
            {LOCK_OPTIONS.map((option) => (
              <option key={option.seconds} value={option.seconds}>
                Lock for {option.label}
              </option>
            ))}
          </select>
          <button className="primary" disabled={busy}>
            Create vault
          </button>
        </form>
      ) : (
        <>
          <div className="stats">
            <div>
              <span className="label">Vault balance</span>
              <span className="value">{sol(vault.balance)} SOL</span>
            </div>
            <div>
              <span className="label">Unlocks in</span>
              <span className="value">{countdown(Number(vault.unlockAt) - now)}</span>
            </div>
          </div>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              run(() => deposit(signer, Number(amount)));
            }}
          >
            <input
              value={amount}
              inputMode="decimal"
              onChange={(e) => setAmount(e.target.value)}
              disabled={busy}
            />
            <button className="primary" disabled={busy || !(Number(amount) > 0)}>
              Deposit SOL
            </button>
            <button type="button" disabled={busy} onClick={() => run(() => withdraw(signer))}>
              Withdraw all
            </button>
          </form>
        </>
      )}

      {error && <p className="error">{error}</p>}

      <h2 className="label">Activity</h2>
      <ul className="activity">
        {activity.length === 0 && <li className="empty">No activity yet.</li>}
        {activity.map((item, i) => (
          <li key={activity.length - i}>{item}</li>
        ))}
      </ul>
    </section>
  );
}
