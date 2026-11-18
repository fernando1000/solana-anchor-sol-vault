use {
    anchor_lang::{
        error::ErrorCode,
        prelude::{Clock, Pubkey},
        solana_program::instruction::Instruction,
        system_program, Event, InstructionData, ToAccountMetas,
    },
    base64::{engine::general_purpose::STANDARD, Engine},
    litesvm::{types::TransactionResult, LiteSVM},
    sol_vault::{accounts, error::VaultError, instruction, Deposited, STATE_SEED, VAULT_SEED},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

const ONE_SOL: u64 = 1_000_000_000;
const ONE_HOUR: i64 = 60 * 60;
const FEE: u64 = 5_000;

// A fresh runtime with our program loaded, and a user with 10 SOL.
fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let program = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/sol_vault.so"
    ));
    svm.add_program(sol_vault::id(), program).unwrap();
    let user = Keypair::new();
    svm.airdrop(&user.pubkey(), 10 * ONE_SOL).unwrap();
    (svm, user)
}

// Same seeds as the program: the state from the owner, the vault from the state.
fn state_pda(owner: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[STATE_SEED, owner.as_ref()], &sol_vault::id()).0
}

fn vault_pda(owner: &Pubkey) -> Pubkey {
    let state = state_pda(owner);
    Pubkey::find_program_address(&[VAULT_SEED, state.as_ref()], &sol_vault::id()).0
}

fn now(svm: &LiteSVM) -> i64 {
    svm.get_sysvar::<Clock>().unix_timestamp
}

// LiteSVM lets us overwrite the Clock sysvar: jump into the future without waiting.
fn move_clock_forward(svm: &mut LiteSVM, seconds: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp += seconds;
    svm.set_sysvar(&clock);
}

fn send(
    svm: &mut LiteSVM,
    signer: &Keypair,
    data: impl InstructionData,
    accounts: impl ToAccountMetas,
) -> TransactionResult {
    let ix = Instruction::new_with_bytes(
        sol_vault::id(),
        &data.data(),
        accounts.to_account_metas(None),
    );
    svm.expire_blockhash();
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&signer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[signer]).unwrap();
    svm.send_transaction(tx)
}

fn initialize(svm: &mut LiteSVM, owner: &Keypair, unlock_at: i64) -> TransactionResult {
    let accounts = accounts::Initialize {
        owner: owner.pubkey(),
        state: state_pda(&owner.pubkey()),
        vault: vault_pda(&owner.pubkey()),
        system_program: system_program::ID,
    };
    send(svm, owner, instruction::Initialize { unlock_at }, accounts)
}

fn deposit(svm: &mut LiteSVM, owner: &Keypair, amount: u64) -> TransactionResult {
    let accounts = accounts::Deposit {
        owner: owner.pubkey(),
        state: state_pda(&owner.pubkey()),
        vault: vault_pda(&owner.pubkey()),
        system_program: system_program::ID,
    };
    send(svm, owner, instruction::Deposit { amount }, accounts)
}

// `vault_owner` picks whose vault: the signer's own, or someone else's.
fn withdraw(svm: &mut LiteSVM, signer: &Keypair, vault_owner: &Pubkey) -> TransactionResult {
    let accounts = accounts::Withdraw {
        owner: signer.pubkey(),
        state: state_pda(vault_owner),
        vault: vault_pda(vault_owner),
        system_program: system_program::ID,
    };
    send(svm, signer, instruction::Withdraw {}, accounts)
}

fn extend_lock(svm: &mut LiteSVM, owner: &Keypair, new_unlock_at: i64) -> TransactionResult {
    let accounts = accounts::ExtendLock {
        owner: owner.pubkey(),
        state: state_pda(&owner.pubkey()),
    };
    send(
        svm,
        owner,
        instruction::ExtendLock { new_unlock_at },
        accounts,
    )
}

fn balance(svm: &LiteSVM, address: &Pubkey) -> u64 {
    svm.get_balance(address).unwrap_or(0)
}

// Anchor writes each event as "Program data: <base64>" in the logs.
fn events<T: Event>(logs: &[String]) -> Vec<T> {
    logs.iter()
        .filter_map(|line| line.strip_prefix("Program data: "))
        .filter_map(|data| STANDARD.decode(data).ok())
        .filter(|bytes| bytes.starts_with(T::DISCRIMINATOR))
        .filter_map(|bytes| T::deserialize(&mut &bytes[T::DISCRIMINATOR.len()..]).ok())
        .collect()
}

fn assert_error(result: TransactionResult, error: impl Into<u32>) {
    let logs = result.unwrap_err().meta.logs.join("\n");
    let number = format!("Error Number: {}.", error.into());
    assert!(logs.contains(&number), "expected {number}\n{logs}");
}

#[test]
fn withdraw_only_after_unlock_time() {
    let (mut svm, user) = setup();
    let vault = vault_pda(&user.pubkey());
    let unlock_at = now(&svm) + ONE_HOUR;
    initialize(&mut svm, &user, unlock_at).unwrap();

    let result = deposit(&mut svm, &user, ONE_SOL).unwrap();
    assert_eq!(balance(&svm, &vault), ONE_SOL);
    let deposited = events::<Deposited>(&result.logs);
    assert_eq!(deposited[0].amount, ONE_SOL);

    let result = withdraw(&mut svm, &user, &user.pubkey());
    assert_error(result, VaultError::StillLocked);

    move_clock_forward(&mut svm, ONE_HOUR);
    let state = state_pda(&user.pubkey());
    let rent = balance(&svm, &state);
    let before = balance(&svm, &user.pubkey());
    withdraw(&mut svm, &user, &user.pubkey()).unwrap();

    assert_eq!(balance(&svm, &vault), 0);
    assert!(svm.get_account(&state).is_none());
    assert_eq!(balance(&svm, &user.pubkey()), before + ONE_SOL + rent - FEE);
}

#[test]
fn unlock_time_in_the_past_fails() {
    let (mut svm, user) = setup();

    let unlock_at = now(&svm) - 1;

    let result = initialize(&mut svm, &user, unlock_at);

    assert_error(result, VaultError::UnlockInThePast);
}

#[test]
fn lock_can_be_extended_but_never_shortened() {
    let (mut svm, user) = setup();
    let unlock_at = now(&svm) + ONE_HOUR;
    initialize(&mut svm, &user, unlock_at).unwrap();

    let result = extend_lock(&mut svm, &user, unlock_at - 1);
    assert_error(result, VaultError::CannotShortenLock);

    extend_lock(&mut svm, &user, unlock_at + ONE_HOUR).unwrap();
}

#[test]
fn zero_deposit_fails() {
    let (mut svm, user) = setup();
    let unlock_at = now(&svm) + ONE_HOUR;
    initialize(&mut svm, &user, unlock_at).unwrap();

    let result = deposit(&mut svm, &user, 0);

    assert_error(result, VaultError::ZeroAmount);
}

#[test]
fn another_wallet_cannot_withdraw_from_my_vault() {
    let (mut svm, user) = setup();
    let attacker = Keypair::new();
    svm.airdrop(&attacker.pubkey(), ONE_SOL).unwrap();
    let unlock_at = now(&svm) + ONE_HOUR;
    initialize(&mut svm, &user, unlock_at).unwrap();
    deposit(&mut svm, &user, ONE_SOL).unwrap();
    move_clock_forward(&mut svm, ONE_HOUR);

    let result = withdraw(&mut svm, &attacker, &user.pubkey());

    assert_error(result, ErrorCode::ConstraintSeeds);
    assert_eq!(balance(&svm, &vault_pda(&user.pubkey())), ONE_SOL);
}
