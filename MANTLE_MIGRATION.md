# Mantle Arsia migration — status

Base: `op-rbuilder/v0.2.14` (paradigmxyz/reth v1.9.3 stack)
Target: `mantle-xyz/reth#v1.9.3-mantle-arsia.1` stack
Branch: `feat/mantle-arsia-v0.2.14`

## Verified

- `cargo build --workspace` — passes
- `cargo test --workspace --no-run` — passes
- 93 / 112 op-rbuilder unit tests pass (83%)

## Dependency mapping

| Crate(s) | Before (v0.2.14) | After (this branch) |
|---|---|---|
| `reth-*` (~50 crates) | paradigmxyz/reth v1.9.3 | mantle-xyz/reth v1.9.3-mantle-arsia.1 |
| `revm` + 10 `revm-*` + `op-revm` | crates.io | mantle-xyz/revm v2.2.2 |
| `revm-inspectors` | crates.io 0.32.0 | mantle-xyz/revm-inspectors v2.2.1 |
| `alloy-evm` + `alloy-op-evm` | crates.io 0.23.0 | mantle-xyz/evm v2.2.1 |
| `op-alloy-*` (consensus / network / rpc-*) | crates.io 0.22.4 | mantle-xyz/op-alloy v2.2.0 |
| `alloy-consensus` + ~20 sibling alloy crates | crates.io 1.0.41 | crates.io `=1.7.3` (exact pin) |
| `alloy-primitives` / `alloy-sol-types` | crates.io 1.4.1 | crates.io `=1.4.1` (exact pin) |
| `vergen` / `vergen-git2` | crates.io 9.0.4 / 1.0.5 | crates.io `=9.0.4` / `=1.0.5` (exact pin) |
| `tdx-attestation-sdk` | branch=main | rev=`0c75c913a8` (pre-regression) |

## Rust toolchain

`1.88.0 → 1.91.0` (required by mantle reth v1.9.3-mantle-arsia.1).

## Source code changes

Only **3 files** in op-rbuilder source touched. All driven by Mantle `TxDeposit`
extension fields:

```rust
// op-alloy v2.2.0 TxDeposit gained two fields:
pub eth_value: u128,                // BVM_ETH L2 mint tag
pub eth_tx_value: Option<u128>,     // BVM_ETH L2 transfer tag
```

### `crates/op-rbuilder/src/builders/flashblocks/payload_handler.rs:303`

Production code that materializes a `DepositTransactionParts` from a decoded
deposit envelope. Wire through `eth_value` / `eth_tx_value`.

### `crates/op-rbuilder/src/tests/framework/driver.rs:136`

Synthetic L1Block-attributes deposit constructed for tests — set both new
fields to zero / `None` (tests don't exercise BVM_ETH bridging).

### `crates/op-rbuilder/src/tests/framework/utils.rs:210, 231`

`ChainDriverExt::fund` / `ChainDriverExt::fund_many` — same treatment.

## Known test failures (19 / 112)

These pass `cargo test --no-run` (compile) but fail at runtime. They surface
as semantic differences between paradigm reth v1.9.3 and mantle reth
v1.9.3-mantle-arsia.1; none are compile errors.

### Category A — flashblocks timing (6 failures)

```
tests::flashblocks::smoke_dynamic_base_flashblocks
tests::flashblocks::smoke_dynamic_unichain_flashblocks
tests::flashblocks::build_at_interval_end_basic_flashblocks
tests::flashblocks::build_at_interval_end_negative_offset_flashblocks
tests::flashblocks::fixed_mode_with_end_buffer_flashblocks
tests::flashblocks::test_flashblocks_number_contract_builder_tx_flashblocks
```

Pattern: off-by-one in expected flashblock count (e.g. `assert_eq!(110, len)`
returned 109). Likely tied to:
- Subtle scheduler-timing difference in mantle reth's payload service.
- Builder marker tx insertion timing.

Investigation: probably need to compare `FlashblockScheduler::new` output
across forks at identical timestamps.

### Category B — max_gas_per_txn enforcement (3 failures)

```
tests::smoke::chain_produces_big_tx_with_gas_limit_standard
tests::smoke::chain_produces_big_tx_with_gas_limit_flashblocks
tests::gas_limiter::gas_limiter_blocks_excessive_usage_*
```

Pattern: block expected to contain `[deposit, valid_tx, builder_marker]` (3 txs)
contains only 2. With `max_gas_per_txn = 25000` enabled, the **builder marker
tx** itself may be exceeding the cap under mantle reth (token_ratio removal in
Arsia changed gas accounting).

Investigation: builder marker tx gas usage under Arsia; consider exempting
builder-self tx from `max_gas_per_txn`, or raising the cap in test.

### Category C — flashtestations (10 failures)

```
tests::flashtestations::test_flashtestations_*
```

All flashtestations tests fail. These exercise TEE-attestation builder txs.
Possibly related to:
- TDX SDK pin (we pinned to 0c75c913a8 which is older than v0.2.14's main HEAD).
- Or builder marker tx counting (same root cause as Category B).

Investigation: check if flashtestations contract address / interface changed
between SDK commits.

## Out of scope for this commit

The following Mantle-specific business logic adaptations were **not** added —
they require test infrastructure validation that's blocked on the 19 failing
tests. Recommended order:

1. **`min_base_fee` tx filtering** — Jovian+ feature. Builder must skip
   txs with `gas_price < min_base_fee`. Touch points:
   - `crates/op-rbuilder/src/builders/flashblocks/best_txs.rs` (cursor filter)
   - or `payload.rs::execute_best_transactions` (post-cursor check)
   - `min_base_fee` is in `OpPayloadJobCtx.block_env_attributes.extra_data`
     (last 8 bytes, big-endian)

2. **Metrics for Mantle-specific events** — add counters for:
   - `mantle_min_base_fee_rejected_total`
   - `mantle_arsia_active` (gauge)
   - `mantle_token_ratio_at_block_start` (deprecated post-Arsia; for migration window)

3. **TokenRatio sanity logging** — log `TokenRatio` from `GasOracleAddr` state
   on every block for monitoring.

4. **`eth_estimateTotalFee` / `eth_sendRawTransactionWithPreconf` confirmation**
   — these RPCs come from reth fork; verify op-rbuilder's `NodeAddOns` setup
   exposes them via the addon's RPC ext.

## Commit log

```
a78e361 test: add Mantle eth_value/eth_tx_value to TxDeposit test fixtures
aff22d5 fix(tdx-quote-provider): pin tdx-attestation-sdk to pre-regression commit
8b62235 feat: switch reth deps to mantle-xyz v1.9.3-mantle-arsia.1
```

## Verification commands

```bash
# Build
cargo build --workspace

# Compile tests (fast)
cargo test --workspace --no-run

# Run tests
cargo test -p op-rbuilder --lib
cargo test -p op-rbuilder --lib -- --skip flashtestations --skip "chain_produces_big"
```
