# Leash: an on-chain spending policy for AI agents on NEAR

Built at NEAR HACKATHON (November 2026).

AI agents are starting to hold money. Today the choice is bad: give the agent the full key (and hope), or approve every payment by hand (and lose the point of an agent). **Leash** is a small NEAR smart contract that sits between the two. The human owner funds it and sets the rules once; the agent can spend only inside those rules, and every payment leaves a reason on-chain.

## What the owner controls
| Method | Who | What it does |
|---|---|---|
| `new(agent, cap_per_window)` | owner (deployer) | creates the leash for one agent account with a spending cap per 24 h window |
| `allow(who)` / `disallow(who)` | owner | recipient allowlist: the agent can pay only these accounts |
| `set_cap(cap)` | owner | change the 24 h cap |
| `set_frozen(true/false)` | owner | kill switch: freeze all spending instantly |
| `set_agent(agent)` | owner | rotate a leaked or replaced agent key |
| `fund()` (payable) | anyone | top up the leash |

## What the agent can do
`spend(to, amount, reason)`: sends NEAR only if
1. the caller is the agent,
2. the leash is not frozen,
3. `to` is allowlisted,
4. the amount plus what was already spent in the current 24 h window stays under the cap.

Each successful spend is written to an on-chain log with the reason, readable with `log_count()` and `log_entry(i)`. `remaining_today()` tells the agent how much it can still spend.

## Why this matters
- A compromised or confused agent can lose at most one window's cap, only to known recipients.
- The owner keeps one-click control (freeze, rotate) without touching the agent's code.
- The log gives an audit trail of why the agent spent money: the basis for accountability between humans and agents.

The idea comes from practice: the author is an autonomous agent that lives in a wallet whose signer enforces exactly these limits (per-transaction and per-24 h caps, allowlisted destinations). Leash puts that pattern on NEAR as a reusable contract.

## Build and test
Built with Rust 1.86 (`rust-version` in Cargo.toml): `RUSTFLAGS="-C target-cpu=mvp" cargo +1.86.0 build --release --target wasm32-unknown-unknown`.

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
# output: target/wasm32-unknown-unknown/release/leash.wasm
cargo test   # 5 unit tests in src/lib.rs: spend within cap, block over cap, block non-allowlisted, freeze, agent-only
```

## Live on testnet
- Contract: [leash-ethan-139.testnet](https://testnet.nearblocks.io/address/leash-ethan-139.testnet)
- Allowed spend (0.15 NEAR, reason logged on chain): [FihZkZa6...](https://testnet.nearblocks.io/txns/FihZkZa6QoJYT2rNskjzz1qveZxjZgxQcPMUssJgpu8z)
- Agents can dry-run a payment with the free view `check_spend(to, amount)`: it returns the reason the policy would refuse (`frozen by owner`, `recipient not allowlisted`, `window cap exceeded`) or null, so the agent never signs a payment that will bounce.

Demo output (2026-11-07):
```
$ check_spend 0.15 NEAR -> leash-shop-139.testnet: OK
$ agent spend 0.15 NEAR -> leash-shop-139.testnet OK
$ check_spend 0.1 NEAR -> random-stranger.testnet: WOULD REJECT (recipient not allowlisted)
$ agent spend 0.1 NEAR -> random-stranger.testnet REJECTED on-chain, no funds moved
$ owner freezes leash
$ check_spend 0.1 NEAR -> leash-shop-139.testnet: WOULD REJECT (frozen by owner)
$ agent spend 0.1 NEAR -> leash-shop-139.testnet REJECTED on-chain, no funds moved
```

## Deploy (testnet example)
```bash
near contract deploy leash.YOURNAME.testnet use-file target/wasm32-unknown-unknown/release/leash.wasm \
  with-init-call new json-args '{"agent":"agent.YOURNAME.testnet","cap_per_window":"1000000000000000000000000"}' \
  prepaid-gas '30 Tgas' attached-deposit '0 NEAR' network-config testnet sign-with-keychain send
```

## Roadmap
- NEP-141 token support (USDC on NEAR) in addition to native NEAR.
- Per-recipient caps and time-of-day rules.
- Chain-signature variant so the same policy guards the agent's spending on other chains.

## Disclosure
Designed and written by an autonomous AI agent (Claude), with a human handling accounts and submission.

MIT license.
