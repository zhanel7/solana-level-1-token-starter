mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup_svm;
use solana_keypair::Keypair;
use solana_message::{AccountMeta, Instruction, Message};
use solana_signer::Signer;
use solana_transaction::Transaction;

const DECIMALS: u8 = 6;

#[test]
fn creates_token_2022_mint() {
    let (mut svm, payer) = setup_svm();
    let program_id = solana_level_1_token_starter::ID;
    let token_program = anchor_spl::token_2022::ID;

    let authority = Keypair::new();
    let mint = Keypair::new();

    let accounts = solana_level_1_token_starter::accounts::CreateToken {
        payer: payer.pubkey(),
        authority: authority.pubkey(),
        mint: mint.pubkey(),
        token_program,
        system_program: anchor_lang::system_program::ID,
    };

    let instruction = Instruction {
        program_id,
        accounts: accounts
            .to_account_metas(None)
            .into_iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer,
                is_writable: meta.is_writable,
            })
            .collect(),
        data: solana_level_1_token_starter::instruction::CreateToken { decimals: DECIMALS }.data(),
    };

    let blockhash = svm.latest_blockhash();
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let transaction = Transaction::new(&[&payer, &authority, &mint], message, blockhash);

    svm.send_transaction(transaction)
        .expect("create_token must succeed");

    let mint_account = svm.get_account(&mint.pubkey()).expect("mint must exist");
    assert_eq!(mint_account.owner, token_program);
    assert!(!mint_account.data.is_empty());
}