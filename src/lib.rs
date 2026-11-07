//! Leash: an on-chain spending policy for an AI agent on NEAR.
//! The owner sets a per-window cap and an allowlist of recipients; the agent
//! can only move funds that satisfy the policy. Owner can freeze or revoke at any time.
use near_sdk::store::{IterableSet, LookupMap};
use near_sdk::{env, near, require, AccountId, NearToken, PanicOnDefault, Promise};

const WINDOW_MS: u64 = 24 * 60 * 60 * 1000;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Leash {
    owner: AccountId,
    agent: AccountId,
    cap_per_window: NearToken,
    window_start_ms: u64,
    spent_in_window: NearToken,
    frozen: bool,
    allowed: IterableSet<AccountId>,
    log: LookupMap<u64, String>,
    log_len: u64,
}

#[near]
impl Leash {
    #[init]
    pub fn new(agent: AccountId, cap_per_window: NearToken) -> Self {
        Self {
            owner: env::predecessor_account_id(),
            agent,
            cap_per_window,
            window_start_ms: env::block_timestamp_ms(),
            spent_in_window: NearToken::from_yoctonear(0),
            frozen: false,
            allowed: IterableSet::new(b"a"),
            log: LookupMap::new(b"l"),
            log_len: 0,
        }
    }

    fn only_owner(&self) {
        require!(env::predecessor_account_id() == self.owner, "owner only");
    }

    #[payable]
    pub fn fund(&mut self) {}

    pub fn allow(&mut self, who: AccountId) { self.only_owner(); self.allowed.insert(who); }
    pub fn disallow(&mut self, who: AccountId) { self.only_owner(); self.allowed.remove(&who); }
    pub fn set_cap(&mut self, cap: NearToken) { self.only_owner(); self.cap_per_window = cap; }
    pub fn set_frozen(&mut self, frozen: bool) { self.only_owner(); self.frozen = frozen; }
    pub fn set_agent(&mut self, agent: AccountId) { self.only_owner(); self.agent = agent; }

    /// Agent asks to pay `amount` to `to`, with a human-readable reason that is logged on chain.
    pub fn spend(&mut self, to: AccountId, amount: NearToken, reason: String) -> Promise {
        require!(env::predecessor_account_id() == self.agent, "agent only");
        require!(!self.frozen, "frozen by owner");
        require!(self.allowed.contains(&to), "recipient not allowlisted");
        let now = env::block_timestamp_ms();
        if now >= self.window_start_ms + WINDOW_MS {
            self.window_start_ms = now;
            self.spent_in_window = NearToken::from_yoctonear(0);
        }
        let total = self.spent_in_window.as_yoctonear() + amount.as_yoctonear();
        require!(total <= self.cap_per_window.as_yoctonear(), "window cap exceeded");
        self.spent_in_window = NearToken::from_yoctonear(total);
        let entry = format!("{}|{}|{}|{}", now, to, amount.as_yoctonear(), reason);
        env::log_str(&entry);
        self.log.insert(self.log_len, entry);
        self.log_len += 1;
        Promise::new(to).transfer(amount)
    }

    pub fn remaining_today(&self) -> NearToken {
        let now = env::block_timestamp_ms();
        if now >= self.window_start_ms + WINDOW_MS { return self.cap_per_window; }
        NearToken::from_yoctonear(self.cap_per_window.as_yoctonear().saturating_sub(self.spent_in_window.as_yoctonear()))
    }
    /// Dry run: returns the reason the policy would reject this spend, or None if it would pass.
    /// Agents call this (free view) before signing, so they never waste gas on a refused payment.
    pub fn check_spend(&self, to: AccountId, amount: NearToken) -> Option<String> {
        if self.frozen { return Some("frozen by owner".into()); }
        if !self.allowed.contains(&to) { return Some("recipient not allowlisted".into()); }
        if amount.as_yoctonear() > self.remaining_today().as_yoctonear() { return Some("window cap exceeded".into()); }
        None
    }
    pub fn is_frozen(&self) -> bool { self.frozen }
    pub fn log_entry(&self, i: u64) -> Option<String> { self.log.get(&i).cloned() }
    pub fn log_count(&self) -> u64 { self.log_len }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn ctx(who: AccountId) { testing_env!(VMContextBuilder::new().predecessor_account_id(who).build()); }
    fn setup() -> Leash {
        ctx(accounts(0));
        let mut c = Leash::new(accounts(1), NearToken::from_near(2));
        c.allow(accounts(2));
        c
    }
    #[test] fn spends_within_cap() {
        let mut c = setup(); ctx(accounts(1));
        c.spend(accounts(2), NearToken::from_near(1), "api".into());
        assert_eq!(c.log_count(), 1);
        assert_eq!(c.remaining_today(), NearToken::from_near(1));
    }
    #[test] #[should_panic(expected = "window cap exceeded")] fn blocks_over_cap() {
        let mut c = setup(); ctx(accounts(1));
        c.spend(accounts(2), NearToken::from_near(3), "x".into());
    }
    #[test] #[should_panic(expected = "recipient not allowlisted")] fn blocks_unlisted() {
        let mut c = setup(); ctx(accounts(1));
        c.spend(accounts(3), NearToken::from_near(1), "x".into());
    }
    #[test] #[should_panic(expected = "frozen by owner")] fn freeze_works() {
        let mut c = setup(); c.set_frozen(true); ctx(accounts(1));
        c.spend(accounts(2), NearToken::from_near(1), "x".into());
    }
    #[test] #[should_panic(expected = "agent only")] fn only_agent() {
        let mut c = setup(); ctx(accounts(3));
        c.spend(accounts(2), NearToken::from_near(1), "x".into());
    }
}
