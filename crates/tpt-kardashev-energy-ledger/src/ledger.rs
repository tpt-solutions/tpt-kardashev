// SPDX-License-Identifier: MIT OR Apache-2.0
//! A double-entry energy ledger with declared sources, stores, sinks and an explicit waste-heat account.
//!
//! Every transfer debits one account and credits another. The total energy in the ledger changes
//! only when an account is opened with funding, and the funding is declared at that moment.
//! `check_conservation` compares the current total with the declared total.
//!
//! Flow rules, which make the sources and sinks explicit:
//! - `Source` accounts are funded at open and may only be debited (energy leaves to the system).
//! - `Store` accounts may be funded, debited and credited. Their balance never goes negative.
//! - `Sink` accounts (useful outputs) and `WasteHeat` accounts may only be credited. They start at zero.

use crate::error::{Error, non_negative};
use crate::units::Joules;

/// The role of an account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AccountKind {
    /// Declared supply. Funded at open, debited only.
    Source,
    /// Internal storage. Funded at open, debited and credited, never negative.
    Store,
    /// Useful output. Credited only, starts at zero.
    Sink,
    /// Dissipated energy. Credited only, starts at zero.
    WasteHeat,
}

impl AccountKind {
    fn may_debit(self) -> bool {
        matches!(self, AccountKind::Source | AccountKind::Store)
    }

    fn may_credit(self) -> bool {
        matches!(
            self,
            AccountKind::Store | AccountKind::Sink | AccountKind::WasteHeat
        )
    }

    fn may_fund(self) -> bool {
        matches!(self, AccountKind::Source | AccountKind::Store)
    }
}

/// Handle to an account in a particular ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccountId(usize);

#[derive(Clone, Copy, Debug)]
struct Account {
    kind: AccountKind,
    name: &'static str,
    balance: f64,
}

/// A ledger with room for `N` accounts. Uses no allocation.
#[derive(Clone, Debug)]
pub struct Ledger<const N: usize> {
    accounts: [Option<Account>; N],
    len: usize,
    declared_total: f64,
}

impl<const N: usize> Default for Ledger<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Ledger<N> {
    /// An empty ledger.
    pub fn new() -> Self {
        Self {
            accounts: [None; N],
            len: 0,
            declared_total: 0.0,
        }
    }

    /// Opens an account with `funding` joules. Only `Source` and `Store` accounts may be funded.
    pub fn open(
        &mut self,
        kind: AccountKind,
        name: &'static str,
        funding: Joules,
    ) -> Result<AccountId, Error> {
        let amount = non_negative(funding.0, "funding")?;
        if amount > 0.0 && !kind.may_fund() {
            return Err(Error::ForbiddenFlow);
        }
        if self.len == N {
            return Err(Error::LedgerFull);
        }
        let id = self.len;
        self.accounts[id] = Some(Account {
            kind,
            name,
            balance: amount,
        });
        self.len += 1;
        self.declared_total += amount;
        Ok(AccountId(id))
    }

    /// Moves `amount` joules from `from` to `to`.
    pub fn transfer(
        &mut self,
        from: AccountId,
        to: AccountId,
        amount: Joules,
    ) -> Result<(), Error> {
        let amount = non_negative(amount.0, "amount")?;
        if from == to {
            return Err(Error::SameAccount);
        }
        let src = self.account(from)?;
        let dst = self.account(to)?;
        if !src.kind.may_debit() || !dst.kind.may_credit() {
            return Err(Error::ForbiddenFlow);
        }
        if src.balance < amount {
            return Err(Error::InsufficientFunds);
        }
        if let Some(a) = self.accounts[from.0].as_mut() {
            a.balance -= amount;
        }
        if let Some(a) = self.accounts[to.0].as_mut() {
            a.balance += amount;
        }
        Ok(())
    }

    /// Current balance of an account.
    pub fn balance(&self, id: AccountId) -> Result<Joules, Error> {
        Ok(Joules(self.account(id)?.balance))
    }

    /// Name given at open.
    pub fn name(&self, id: AccountId) -> Result<&'static str, Error> {
        Ok(self.account(id)?.name)
    }

    /// Sum of all balances.
    pub fn total(&self) -> Joules {
        Joules(
            self.accounts[..self.len]
                .iter()
                .flatten()
                .map(|a| a.balance)
                .sum(),
        )
    }

    /// Total energy declared at open, summed over all funded accounts.
    pub fn declared_total(&self) -> Joules {
        Joules(self.declared_total)
    }

    /// Sum of balances over accounts of one kind.
    pub fn total_of(&self, kind: AccountKind) -> Joules {
        Joules(
            self.accounts[..self.len]
                .iter()
                .flatten()
                .filter(|a| a.kind == kind)
                .map(|a| a.balance)
                .sum(),
        )
    }

    /// Checks that the current total matches the declared total within relative tolerance `1e-12`.
    pub fn check_conservation(&self) -> Result<(), Error> {
        let diff = (self.total().0 - self.declared_total).abs();
        if diff <= 1e-12 * self.declared_total.abs() {
            Ok(())
        } else {
            Err(Error::NotConserved)
        }
    }

    fn account(&self, id: AccountId) -> Result<&Account, Error> {
        match self.accounts.get(id.0) {
            Some(Some(a)) if id.0 < self.len => Ok(a),
            _ => Err(Error::UnknownAccount),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_moves_energy_and_conserves_total() {
        let mut l: Ledger<4> = Ledger::new();
        let src = l
            .open(AccountKind::Source, "supply", Joules(100.0))
            .expect("ok");
        let store = l
            .open(AccountKind::Store, "buffer", Joules(0.0))
            .expect("ok");
        let waste = l
            .open(AccountKind::WasteHeat, "heat", Joules(0.0))
            .expect("ok");
        l.transfer(src, store, Joules(60.0)).expect("ok");
        l.transfer(store, waste, Joules(25.0)).expect("ok");
        assert_eq!(l.balance(store).expect("ok").0, 35.0);
        assert_eq!(l.total_of(AccountKind::WasteHeat).0, 25.0);
        assert!(l.check_conservation().is_ok());
    }

    #[test]
    fn flow_rules_are_enforced() {
        let mut l: Ledger<4> = Ledger::new();
        let src = l
            .open(AccountKind::Source, "supply", Joules(10.0))
            .expect("ok");
        let sink = l.open(AccountKind::Sink, "work", Joules(0.0)).expect("ok");
        let waste = l
            .open(AccountKind::WasteHeat, "heat", Joules(0.0))
            .expect("ok");
        assert_eq!(
            l.transfer(sink, src, Joules(1.0)),
            Err(Error::ForbiddenFlow)
        );
        assert_eq!(
            l.transfer(waste, sink, Joules(1.0)),
            Err(Error::ForbiddenFlow)
        );
        assert_eq!(l.transfer(src, src, Joules(1.0)), Err(Error::SameAccount));
        assert_eq!(
            l.transfer(src, sink, Joules(11.0)),
            Err(Error::InsufficientFunds)
        );
        assert_eq!(
            l.open(AccountKind::Sink, "bad", Joules(5.0)),
            Err(Error::ForbiddenFlow)
        );
        assert_eq!(
            l.transfer(src, sink, Joules(-1.0)),
            Err(Error::Negative { name: "amount" })
        );
    }

    #[test]
    fn capacity_and_unknown_ids() {
        let mut l: Ledger<1> = Ledger::new();
        let a = l.open(AccountKind::Store, "a", Joules(1.0)).expect("ok");
        assert_eq!(
            l.open(AccountKind::Store, "b", Joules(1.0)),
            Err(Error::LedgerFull)
        );
        assert_eq!(l.balance(AccountId(5)), Err(Error::UnknownAccount));
        assert_eq!(l.name(a), Ok("a"));
    }
}
