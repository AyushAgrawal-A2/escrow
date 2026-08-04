# escrow

SPL token escrow: a maker deposits `amount` of mint A and names a price of `receive` of mint B;
any taker who pays the price gets the deposit; the maker can cancel and get the deposit back.

Written from scratch as an Anchor learning build. Not related to the `blueshift_escrow` repo in
this account — that solves the Blueshift challenge against its fixed program id `2222...`; this
one has its own program id, uses `token_interface` throughout
(works with both spl-token and Token-2022), creates destination ATAs in the handler with
`create_idempotent` instead of `init_if_needed` constraints, and is tested in Rust with LiteSVM.

Anchor `1.1.2` (`anchor-lang` + `anchor-spl`), Rust toolchain `1.96.0`.
Program ID (also in `Anchor.toml`): `3Dz2pbsazJTFFnBuEtN2RpAeRC3z1c6qmNn9Z6hYwrBG`.

## Instructions

| Instruction | Signer | What it does |
|---|---|---|
| `make(id, amount, receive)` | maker | Require `amount > 0` and `receive > 0`, init escrow PDA + vault ATA, transfer `amount` of mint A into the vault |
| `take(id)` | taker | Pay `receive` of mint B to the maker, receive the vault contents, close vault + escrow |
| `refund(id)` | maker | Return the vault contents to the maker, close vault + escrow |

PDAs and token accounts:

| Account | Derivation |
|---|---|
| `escrow` | `["escrow", maker, id.to_le_bytes()]` (the `u64` id allows concurrent escrows per maker) |
| `escrow_vault` | ATA of mint A owned by the escrow PDA |

## State

`Escrow`, 113 bytes = 8 (discriminator) + 105 (`InitSpace`):

| field | type | size |
|---|---|---|
| maker | Pubkey | 32 |
| mint_a | Pubkey | 32 |
| mint_b | Pubkey | 32 |
| receive | u64 | 8 |
| bump | u8 | 1 |

The deposited amount is deliberately not stored — `take` and `refund` move
`escrow_vault.amount`, the vault's live balance, so tokens sent straight to the vault ATA go to
whoever settles the escrow.

## Validation

- `take`/`refund` re-derive the escrow PDA from seeds with the stored bump, plus
  `has_one = maker` / `mint_a` / `mint_b`; `close = maker` returns rent either way.
- In `take`, `taker_a_ata` and `maker_b_ata` are `UncheckedAccount`s: the ATA program's own
  address derivation inside the `create_idempotent` CPI is what pins them to the right
  owner/mint, so a wrong account fails the CPI rather than an Anchor constraint.
- Both mints must be owned by the same token program (`mint::token_program = token_program`);
  a legacy-token mint can't be paired with a Token-2022 mint.
- `make` rejects `amount == 0` and `receive == 0` (`EscrowError::InvalidAmount`), so every open
  escrow holds a real deposit and names a real price.

## Build and test

```bash
anchor build   # produces target/deploy/escrow.so
cargo test     # LiteSVM: make→take and make→refund happy paths
```

The tests `include_bytes!` the `.so` from `target/deploy/`, so `anchor build` must run first.
`anchor test` does both (`[scripts] test = "cargo test"`, no local validator needed).
