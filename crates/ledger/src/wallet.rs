use crate::{Currency, LedgerError, Money};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Wallet {
    id: u64,
    name: String,
    currency: Currency,
}
impl Wallet {
    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn currency(&self) -> Currency {
        self.currency
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    Opening,
    Income,
    Expense,
    Adjustment,
}
impl EntryKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Opening => "初始余额",
            Self::Income => "收入",
            Self::Expense => "支出",
            Self::Adjustment => "余额校正",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    id: u64,
    wallet_id: u64,
    date: NaiveDate,
    time: String,
    kind: EntryKind,
    amount: Money,
    balance: Money,
    // Exchange rate at the operation freezes historical flow totals.
    rate_micros: u64,
}
impl Entry {
    pub fn id(&self) -> u64 {
        self.id
    }
    pub fn wallet_id(&self) -> u64 {
        self.wallet_id
    }
    pub fn date(&self) -> NaiveDate {
        self.date
    }
    pub fn time(&self) -> &str {
        &self.time
    }
    pub fn kind(&self) -> EntryKind {
        self.kind
    }
    pub fn amount(&self) -> Money {
        self.amount
    }
    pub fn balance(&self) -> Money {
        self.balance
    }
    pub fn cny_amount(&self) -> i64 {
        convert(self.amount.cents(), self.rate_micros)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rate {
    date: NaiveDate,
    synced_on: NaiveDate,
    micros: u64,
}
impl Rate {
    pub fn new(date: NaiveDate, synced_on: NaiveDate, micros: u64) -> Result<Self, LedgerError> {
        if date > synced_on || !(100_000..=100_000_000).contains(&micros) {
            return Err(LedgerError::InvalidData);
        }
        Ok(Self {
            date,
            synced_on,
            micros,
        })
    }
    pub fn date(&self) -> NaiveDate {
        self.date
    }
    pub fn synced_on(&self) -> NaiveDate {
        self.synced_on
    }
    pub fn value(&self) -> f64 {
        self.micros as f64 / 1_000_000.0
    }
}

pub(crate) fn convert(cents: i64, micros: u64) -> i64 {
    ((i128::from(cents) * i128::from(micros) + 500_000) / 1_000_000) as i64
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ledger {
    version: u32,
    next_id: u64,
    wallets: Vec<Wallet>,
    entries: Vec<Entry>,
    rates: Vec<Rate>,
    dark: bool,
}
impl Default for Ledger {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            wallets: vec![],
            entries: vec![],
            rates: vec![],
            dark: false,
        }
    }
}
impl Ledger {
    pub fn wallets(&self) -> &[Wallet] {
        &self.wallets
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn wallet(&self, id: u64) -> Option<&Wallet> {
        self.wallets.iter().find(|w| w.id == id)
    }
    /// Permanently removes a wallet and its operations; IDs are never reused.
    pub fn delete_wallet(&mut self, id: u64) -> Result<(), LedgerError> {
        self.wallet(id).ok_or(LedgerError::MissingWallet)?;
        self.wallets.retain(|wallet| wallet.id != id);
        self.entries.retain(|entry| entry.wallet_id != id);
        Ok(())
    }
    pub fn is_dark(&self) -> bool {
        self.dark
    }
    pub fn set_dark(&mut self, dark: bool) {
        self.dark = dark;
    }
    pub fn latest_rate(&self) -> Option<&Rate> {
        self.rates.iter().max_by_key(|r| (r.synced_on, r.date))
    }
    pub fn set_rate(&mut self, rate: Rate) {
        self.rates.retain(|r| r.synced_on != rate.synced_on);
        self.rates.push(rate);
    }
    pub fn balance(&self, id: u64) -> Money {
        self.entries
            .iter()
            .rev()
            .find(|e| e.wallet_id == id)
            .map(|e| e.balance)
            .unwrap_or_default()
    }
    pub fn last_entry(&self, id: u64) -> Option<&Entry> {
        self.entries.iter().rev().find(|e| e.wallet_id == id)
    }
    pub fn balance_on(&self, id: u64, date: NaiveDate) -> Money {
        self.entries
            .iter()
            .rev()
            .find(|e| e.wallet_id == id && e.date <= date)
            .map(|e| e.balance)
            .unwrap_or_default()
    }
    pub fn first_date(&self) -> Option<NaiveDate> {
        self.entries.first().map(|e| e.date)
    }
    fn rate_for(&self, currency: Currency, date: NaiveDate) -> Result<u64, LedgerError> {
        if currency == Currency::Cny {
            return Ok(1_000_000);
        }
        self.rates
            .iter()
            .filter(|r| r.synced_on <= date)
            .max_by_key(|r| r.synced_on)
            .map(|r| r.micros)
            .ok_or(LedgerError::MissingRate)
    }
    pub fn total_on(&self, date: NaiveDate) -> Result<i64, LedgerError> {
        self.wallets.iter().try_fold(0i64, |total, w| {
            let balance = self.balance_on(w.id, date).cents();
            let converted = if balance == 0 {
                0
            } else {
                convert(balance, self.rate_for(w.currency, date)?)
            };
            total.checked_add(converted).ok_or(LedgerError::Overflow)
        })
    }
    pub fn cny_balance(&self, id: u64, date: NaiveDate) -> Result<i64, LedgerError> {
        let w = self.wallet(id).ok_or(LedgerError::MissingWallet)?;
        Ok(convert(
            self.balance_on(id, date).cents(),
            self.rate_for(w.currency, date)?,
        ))
    }
    fn check_date(&self, date: NaiveDate) -> Result<(), LedgerError> {
        if self.entries.last().is_some_and(|e| e.date > date) {
            return Err(LedgerError::ClockReversed);
        }
        Ok(())
    }
    pub fn create(
        &mut self,
        name: &str,
        currency: Currency,
        initial: Money,
        date: NaiveDate,
        time: &str,
    ) -> Result<u64, LedgerError> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 40 {
            return Err(LedgerError::InvalidName);
        }
        self.check_date(date)?;
        let rate_micros = self.rate_for(currency, date)?;
        let id = self.next_id;
        self.next_id += 1;
        self.wallets.push(Wallet {
            id,
            name: name.into(),
            currency,
        });
        self.entries.push(Entry {
            id: self.next_id,
            wallet_id: id,
            date,
            time: time.into(),
            kind: EntryKind::Opening,
            amount: initial,
            balance: initial,
            rate_micros,
        });
        self.next_id += 1;
        Ok(id)
    }
    pub fn record(
        &mut self,
        id: u64,
        amount: Money,
        income: bool,
        date: NaiveDate,
        time: &str,
    ) -> Result<(), LedgerError> {
        self.check_date(date)?;
        let wallet = self.wallet(id).ok_or(LedgerError::MissingWallet)?;
        let rate_micros = self.rate_for(wallet.currency, date)?;
        let previous = self.balance(id).cents();
        let (kind, delta, balance) = if income {
            if amount.cents() == 0 {
                return Err(LedgerError::EmptyIncome);
            }
            let balance = Money::from_cents(previous + amount.cents())?;
            (EntryKind::Income, amount, balance)
        } else if amount.cents() < previous {
            (
                EntryKind::Expense,
                Money::from_cents(previous - amount.cents())?,
                amount,
            )
        } else {
            (
                EntryKind::Adjustment,
                Money::from_cents(amount.cents() - previous)?,
                amount,
            )
        };
        self.entries.push(Entry {
            id: self.next_id,
            wallet_id: id,
            date,
            time: time.into(),
            kind,
            amount: delta,
            balance,
            rate_micros,
        });
        self.next_id += 1;
        Ok(())
    }
    pub(crate) fn validate(&self) -> Result<(), LedgerError> {
        let mut replay = Self {
            rates: self.rates.clone(),
            ..Self::default()
        };
        for rate in &self.rates {
            Rate::new(rate.date, rate.synced_on, rate.micros)?;
        }
        if self.version != 1 || self.wallets.len() > 10_000 {
            return Err(LedgerError::InvalidData);
        }
        let mut ids = std::collections::HashSet::new();
        for wallet in &self.wallets {
            if !ids.insert(wallet.id)
                || wallet.id >= self.next_id
                || wallet.name.trim().is_empty()
                || wallet.name.chars().count() > 40
            {
                return Err(LedgerError::InvalidData);
            }
        }
        let mut opened = std::collections::HashSet::new();
        for e in &self.entries {
            if !ids.insert(e.id)
                || e.id >= self.next_id
                || self.wallet(e.wallet_id).is_none()
                || e.rate_micros == 0
                || e.rate_micros > 100_000_000
            {
                return Err(LedgerError::InvalidData);
            }
            Money::from_cents(e.amount.cents())?;
            Money::from_cents(e.balance.cents())?;
            replay.check_date(e.date)?;
            let old = replay.balance(e.wallet_id).cents();
            let expected = match e.kind {
                EntryKind::Opening if opened.insert(e.wallet_id) => e.amount.cents(),
                EntryKind::Opening => return Err(LedgerError::InvalidData),
                _ if !opened.contains(&e.wallet_id) => return Err(LedgerError::InvalidData),
                EntryKind::Expense => old - e.amount.cents(),
                EntryKind::Income | EntryKind::Adjustment => old + e.amount.cents(),
            };
            if expected != e.balance.cents() {
                return Err(LedgerError::InvalidData);
            }
            replay.entries.push(e.clone());
        }
        if opened.len() != self.wallets.len() {
            return Err(LedgerError::InvalidData);
        }
        Ok(())
    }
}
