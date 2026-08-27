#![allow(clippy::result_large_err)]

use anchor_lang::{AccountDeserialize, Ids, InstructionData, ToAccountMetas};
use anchor_spl::{
    associated_token::{get_associated_token_address_with_program_id, ID as ASSOCIATED_TOKEN_ID},
    token_2022,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use litesvm::{types::TransactionResult, LiteSVM};
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction, Message};
use solana_signer::Signer;
use solana_transaction::Transaction;
use std::{fs, path::PathBuf};

const DECIMALS: u8 = 6;
const INITIAL_SUPPLY: u64 = 5_000_000;
const BURN_AMOUNT: u64 = 1_250_000;

fn program_bytes() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy/solana_level_1_token_starter.so");
    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "Build the program with `anchor build` before running tests. Could not read {}: {error}",
            path.display()
        )
    })
}

fn instruction<A: ToAccountMetas, D: InstructionData>(accounts: A, data: D) -> Instruction {
    Instruction {
        program_id: solana_level_1_token_starter::ID,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: data.data(),
    }
}

fn send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    instruction: Instruction,
    additional_signers: &[&Keypair],
) -> TransactionResult {
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let mut signers: Vec<&dyn Signer> = vec![payer];
    signers.extend(
        additional_signers
            .iter()
            .map(|signer| *signer as &dyn Signer),
    );
    let transaction = Transaction::new(&signers, message, svm.latest_blockhash());
    svm.send_transaction(transaction)
}

fn mint_state(svm: &LiteSVM, address: &anchor_lang::prelude::Pubkey) -> Mint {
    let account = svm.get_account(address).expect("mint account must exist");
    Mint::try_deserialize(&mut account.data.as_slice()).expect("mint must deserialize")
}

fn token_state(svm: &LiteSVM, address: &anchor_lang::prelude::Pubkey) -> TokenAccount {
    let account = svm.get_account(address).expect("token account must exist");
    TokenAccount::try_deserialize(&mut account.data.as_slice())
        .expect("token account must deserialize")
}

struct Fixture {
    svm: LiteSVM,
    payer: Keypair,
    authority: Keypair,
    wrong_authority: Keypair,
    mint: Keypair,
    other_mint: Keypair,
    token_account: anchor_lang::prelude::Pubkey,
}

impl Fixture {
    fn new() -> Self {
        let program_id = solana_level_1_token_starter::ID;
        let token_program = token_2022::ID;
        let mut svm = LiteSVM::new();
        svm.add_program(program_id, &program_bytes())
            .expect("program must load");

        let payer = Keypair::new();
        let authority = Keypair::new();
        let wrong_authority = Keypair::new();
        let mint = Keypair::new();
        let other_mint = Keypair::new();
        svm.airdrop(&payer.pubkey(), 5_000_000_000)
            .expect("payer airdrop must succeed");
        svm.airdrop(&authority.pubkey(), 1_000_000)
            .expect("authority account must exist");
        svm.airdrop(&wrong_authority.pubkey(), 1_000_000)
            .expect("wrong authority account must exist");

        for mint_keypair in [&mint, &other_mint] {
            let create_mint = instruction(
                solana_level_1_token_starter::accounts::CreateToken {
                    payer: payer.pubkey(),
                    authority: authority.pubkey(),
                    mint: mint_keypair.pubkey(),
                    token_program,
                    system_program: anchor_lang::system_program::ID,
                },
                solana_level_1_token_starter::instruction::CreateToken { decimals: DECIMALS },
            );
            send(&mut svm, &payer, create_mint, &[&authority, mint_keypair])
                .expect("mint creation must succeed");
        }

        let token_account = get_associated_token_address_with_program_id(
            &authority.pubkey(),
            &mint.pubkey(),
            &token_program,
        );
        let create_token_account = instruction(
            solana_level_1_token_starter::accounts::CreateTokenAccount {
                payer: payer.pubkey(),
                owner: authority.pubkey(),
                mint: mint.pubkey(),
                token_account,
                token_program,
                associated_token_program: ASSOCIATED_TOKEN_ID,
                system_program: anchor_lang::system_program::ID,
            },
            solana_level_1_token_starter::instruction::CreateTokenAccount {},
        );
        send(&mut svm, &payer, create_token_account, &[])
            .expect("token account creation must succeed");

        let mint_tokens = instruction(
            solana_level_1_token_starter::accounts::MintTokens {
                authority: authority.pubkey(),
                mint: mint.pubkey(),
                destination: token_account,
                token_program,
            },
            solana_level_1_token_starter::instruction::MintTokens {
                amount: INITIAL_SUPPLY,
            },
        );
        send(&mut svm, &payer, mint_tokens, &[&authority])
            .expect("minting fixture tokens must succeed");

        Self {
            svm,
            payer,
            authority,
            wrong_authority,
            mint,
            other_mint,
            token_account,
        }
    }

    fn burn_with(
        &mut self,
        authority: &Keypair,
        mint: anchor_lang::prelude::Pubkey,
        amount: u64,
    ) -> TransactionResult {
        self.burn_with_program(authority, mint, amount, token_2022::ID)
    }

    fn burn_with_program(
        &mut self,
        authority: &Keypair,
        mint: anchor_lang::prelude::Pubkey,
        amount: u64,
        token_program: anchor_lang::prelude::Pubkey,
    ) -> TransactionResult {
        let burn = instruction(
            solana_level_1_token_starter::accounts::BurnTokens {
                authority: authority.pubkey(),
                mint,
                token_account: self.token_account,
                token_program,
            },
            solana_level_1_token_starter::instruction::BurnTokens { amount },
        );
        send(&mut self.svm, &self.payer, burn, &[authority])
    }

    fn protected_state(&self) -> (Vec<u8>, Vec<u8>) {
        (
            self.svm
                .get_account(&self.mint.pubkey())
                .expect("mint must exist")
                .data,
            self.svm
                .get_account(&self.token_account)
                .expect("token account must exist")
                .data,
        )
    }
}

#[test]
fn successful_burn_reduces_balance_and_supply_by_the_same_amount() {
    let mut fixture = Fixture::new();
    let supply_before = mint_state(&fixture.svm, &fixture.mint.pubkey()).supply;
    let balance_before = token_state(&fixture.svm, &fixture.token_account).amount;

    let authority = fixture.authority.insecure_clone();
    let mint = fixture.mint.pubkey();
    fixture
        .burn_with(&authority, mint, BURN_AMOUNT)
        .expect("burn must succeed");

    let supply_after = mint_state(&fixture.svm, &fixture.mint.pubkey()).supply;
    let balance_after = token_state(&fixture.svm, &fixture.token_account).amount;
    assert_eq!(supply_before - supply_after, BURN_AMOUNT);
    assert_eq!(balance_before - balance_after, BURN_AMOUNT);
    assert_eq!(supply_before - supply_after, balance_before - balance_after);
}

#[test]
fn zero_amount_returns_expected_program_error_without_state_changes() {
    let mut fixture = Fixture::new();
    let before = fixture.protected_state();
    let authority = fixture.authority.insecure_clone();
    let mint = fixture.mint.pubkey();

    let failure = fixture
        .burn_with(&authority, mint, 0)
        .expect_err("zero burn must fail");

    assert!(
        format!("{:?}", failure.err).contains("Custom(6000)"),
        "unexpected error: {:?}",
        failure.err
    );
    assert!(
        failure
            .meta
            .logs
            .iter()
            .any(|log| log.contains("Amount must be greater than zero")),
        "program log must contain the custom error message"
    );
    assert_eq!(fixture.protected_state(), before);
}

#[test]
fn invalid_authority_mint_token_program_and_balance_are_atomic() {
    let mut fixture = Fixture::new();

    let before_wrong_authority = fixture.protected_state();
    let wrong_authority = fixture.wrong_authority.insecure_clone();
    let mint = fixture.mint.pubkey();
    fixture
        .burn_with(&wrong_authority, mint, 1)
        .expect_err("wrong authority must fail");
    assert_eq!(fixture.protected_state(), before_wrong_authority);

    let before_wrong_mint = fixture.protected_state();
    let authority = fixture.authority.insecure_clone();
    let other_mint = fixture.other_mint.pubkey();
    fixture
        .burn_with(&authority, other_mint, 1)
        .expect_err("different mint must fail");
    assert_eq!(fixture.protected_state(), before_wrong_mint);

    let before_wrong_token_program = fixture.protected_state();
    let authority = fixture.authority.insecure_clone();
    let mint = fixture.mint.pubkey();
    let wrong_token_program = TokenInterface::ids()
        .iter()
        .copied()
        .find(|program_id| *program_id != token_2022::ID)
        .expect("interface must support the original Token Program");
    fixture
        .burn_with_program(&authority, mint, 1, wrong_token_program)
        .expect_err("token program that does not own the accounts must fail");
    assert_eq!(fixture.protected_state(), before_wrong_token_program);

    let before_insufficient_balance = fixture.protected_state();
    let authority = fixture.authority.insecure_clone();
    let mint = fixture.mint.pubkey();
    fixture
        .burn_with(&authority, mint, INITIAL_SUPPLY + 1)
        .expect_err("insufficient balance must fail");
    assert_eq!(fixture.protected_state(), before_insufficient_balance);
}
