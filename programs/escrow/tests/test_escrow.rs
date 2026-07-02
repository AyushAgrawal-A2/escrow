use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    anchor_spl::{associated_token, token},
    litesvm::LiteSVM,
    litesvm_token::{get_spl_account, CreateAssociatedTokenAccount, CreateMint, MintTo},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

#[test]
fn test_escrow_make_take() {
    let program_id = escrow::id();
    let mint_authority = Keypair::new();
    let maker = Keypair::new();
    let taker = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/escrow.so"));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&maker.pubkey(), 1_000_000_000).unwrap();
    svm.airdrop(&taker.pubkey(), 1_000_000_000).unwrap();

    let id = 1u64;
    let amount = 500_000;
    let receive = 1_000_000;
    let (escrow, escrow_bump) = Pubkey::find_program_address(
        &[
            escrow::constants::ESCROW_SEED,
            maker.pubkey().as_ref(),
            id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );
    let mint_a = CreateMint::new(&mut svm, &mint_authority)
        .authority(&mint_authority.pubkey())
        .decimals(6)
        .send()
        .unwrap();
    let mint_b = CreateMint::new(&mut svm, &mint_authority)
        .authority(&mint_authority.pubkey())
        .decimals(7)
        .send()
        .unwrap();

    let maker_a_ata = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
        .owner(&maker.pubkey())
        .send()
        .unwrap();
    let maker_b_ata = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_b)
        .owner(&maker.pubkey())
        .send()
        .unwrap();
    let taker_a_ata = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_a)
        .owner(&taker.pubkey())
        .send()
        .unwrap();
    let taker_b_ata = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_b)
        .owner(&taker.pubkey())
        .send()
        .unwrap();
    let escrow_vault = associated_token::get_associated_token_address_with_program_id(
        &escrow,
        &mint_a,
        &token::ID,
    );

    MintTo::new(&mut svm, &mint_authority, &mint_a, &maker_a_ata, 10_000_000)
        .send()
        .unwrap();
    MintTo::new(
        &mut svm,
        &mint_authority,
        &mint_b,
        &taker_b_ata,
        100_000_000,
    )
    .send()
    .unwrap();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &escrow::instruction::Make {
            id,
            amount,
            receive,
        }
        .data(),
        escrow::accounts::Make {
            maker: maker.pubkey(),
            escrow,
            maker_a_ata,
            escrow_vault,
            mint_a,
            mint_b,
            associated_token_program: associated_token::ID,
            token_program: token::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&maker.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&maker]).unwrap();
    svm.send_transaction(tx).unwrap();
    let escrow_account = svm.get_account(&escrow).unwrap();
    let mut escrow_data = escrow_account.data.as_slice();
    let escrow_state = escrow::state::Escrow::try_deserialize(&mut escrow_data).unwrap();
    assert_eq!(escrow_state.maker, maker.pubkey());
    assert_eq!(escrow_state.mint_a, mint_a);
    assert_eq!(escrow_state.mint_b, mint_b);
    assert_eq!(escrow_state.receive, receive);
    assert_eq!(escrow_state.bump, escrow_bump);
    let maker_a_ata_state: litesvm_token::spl_token::state::Account =
        get_spl_account(&svm, &maker_a_ata).unwrap();
    assert_eq!(maker_a_ata_state.amount, 10_000_000 - amount);
    let escrow_vault_state: litesvm_token::spl_token::state::Account =
        get_spl_account(&svm, &escrow_vault).unwrap();
    assert_eq!(escrow_vault_state.amount, amount);
    assert_eq!(escrow_vault_state.mint, mint_a);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &escrow::instruction::Take { id }.data(),
        escrow::accounts::Take {
            taker: taker.pubkey(),
            maker: maker.pubkey(),
            escrow,
            escrow_vault,
            taker_a_ata,
            taker_b_ata,
            maker_b_ata,
            mint_a,
            mint_b,
            associated_token_program: associated_token::ID,
            token_program: token::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&taker.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&taker]).unwrap();
    svm.send_transaction(tx).unwrap();
    assert!(svm.get_account(&escrow).is_none());
    assert!(svm.get_account(&escrow_vault).is_none());
    let maker_b_ata_state: litesvm_token::spl_token::state::Account =
        get_spl_account(&svm, &maker_b_ata).unwrap();
    assert_eq!(maker_b_ata_state.amount, receive);
    let taker_b_ata_state: litesvm_token::spl_token::state::Account =
        get_spl_account(&svm, &taker_b_ata).unwrap();
    assert_eq!(taker_b_ata_state.amount, 100_000_000 - receive);
    assert_eq!(taker_b_ata_state.mint, mint_b);
    let taker_a_ata_state: litesvm_token::spl_token::state::Account =
        get_spl_account(&svm, &taker_a_ata).unwrap();
    assert_eq!(taker_a_ata_state.amount, amount);
}

#[test]
fn test_escrow_make_refund() {
    let program_id = escrow::id();
    let mint_authority = Keypair::new();
    let maker = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(env!("CARGO_TARGET_TMPDIR"), "/../deploy/escrow.so"));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();
    svm.airdrop(&maker.pubkey(), 1_000_000_000).unwrap();

    let id = 1u64;
    let amount = 500_000;
    let receive = 1_000_000;
    let (escrow, escrow_bump) = Pubkey::find_program_address(
        &[
            escrow::constants::ESCROW_SEED,
            maker.pubkey().as_ref(),
            id.to_le_bytes().as_ref(),
        ],
        &program_id,
    );
    let mint_a = CreateMint::new(&mut svm, &mint_authority)
        .authority(&mint_authority.pubkey())
        .decimals(6)
        .send()
        .unwrap();
    let mint_b = CreateMint::new(&mut svm, &mint_authority)
        .authority(&mint_authority.pubkey())
        .decimals(7)
        .send()
        .unwrap();

    let maker_a_ata = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
        .owner(&maker.pubkey())
        .send()
        .unwrap();
    let escrow_vault = associated_token::get_associated_token_address_with_program_id(
        &escrow,
        &mint_a,
        &token::ID,
    );

    MintTo::new(&mut svm, &mint_authority, &mint_a, &maker_a_ata, 10_000_000)
        .send()
        .unwrap();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &escrow::instruction::Make {
            id,
            amount,
            receive,
        }
        .data(),
        escrow::accounts::Make {
            maker: maker.pubkey(),
            escrow,
            maker_a_ata,
            escrow_vault,
            mint_a,
            mint_b,
            associated_token_program: associated_token::ID,
            token_program: token::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&maker.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&maker]).unwrap();
    svm.send_transaction(tx).unwrap();
    let escrow_account = svm.get_account(&escrow).unwrap();
    let mut escrow_data = escrow_account.data.as_slice();
    let escrow_state = escrow::state::Escrow::try_deserialize(&mut escrow_data).unwrap();
    assert_eq!(escrow_state.maker, maker.pubkey());
    assert_eq!(escrow_state.mint_a, mint_a);
    assert_eq!(escrow_state.mint_b, mint_b);
    assert_eq!(escrow_state.receive, receive);
    assert_eq!(escrow_state.bump, escrow_bump);
    let maker_a_ata_state: litesvm_token::spl_token::state::Account =
        get_spl_account(&svm, &maker_a_ata).unwrap();
    assert_eq!(maker_a_ata_state.amount, 10_000_000 - amount);
    let escrow_vault_state: litesvm_token::spl_token::state::Account =
        get_spl_account(&svm, &escrow_vault).unwrap();
    assert_eq!(escrow_vault_state.amount, amount);
    assert_eq!(escrow_vault_state.mint, mint_a);

    let instruction = Instruction::new_with_bytes(
        program_id,
        &escrow::instruction::Refund { id }.data(),
        escrow::accounts::Refund {
            maker: maker.pubkey(),
            escrow,
            escrow_vault,
            maker_a_ata,
            mint_a,
            associated_token_program: associated_token::ID,
            token_program: token::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );
    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&maker.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&maker]).unwrap();
    svm.send_transaction(tx).unwrap();
    let maker_a_ata_state: litesvm_token::spl_token::state::Account =
        get_spl_account(&svm, &maker_a_ata).unwrap();
    assert_eq!(maker_a_ata_state.amount, 10_000_000);
    assert!(svm.get_account(&escrow).is_none());
    assert!(svm.get_account(&escrow_vault).is_none());
}
