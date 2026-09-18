use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_signer::Signer;
use std::{fs, path::PathBuf};

pub fn program_bytes() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy/solana_level_1_token_starter.so");
    fs::read(&path).unwrap_or_else(|error| {
        panic!(
            "Build the program with `anchor build` before running tests. Could not read {}: {error}",
            path.display()
        )
    })
}

pub fn setup_svm() -> (LiteSVM, Keypair) {
    let program_id = solana_level_1_token_starter::ID;
    let mut svm = LiteSVM::new();
    svm.add_program(program_id, &program_bytes())
        .expect("program must load");

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .expect("airdrop must succeed");

    (svm, payer)
}