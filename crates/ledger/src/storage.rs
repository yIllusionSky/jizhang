use crate::{Ledger, LedgerError};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

/// One writer per app. Atomic replacement occurs only after a complete fsync.
pub struct Store {
    path: PathBuf,
}
impl Store {
    pub fn new(directory: impl AsRef<Path>) -> Self {
        Self {
            path: directory.as_ref().join("wallets-v1.json"),
        }
    }
    pub fn load(&self) -> Result<Ledger, LedgerError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Ledger::default()),
            Err(e) => return Err(e.into()),
        };
        let ledger: Ledger = serde_json::from_slice(&bytes)?;
        ledger.validate()?;
        Ok(ledger)
    }
    pub fn save(&self, ledger: &Ledger) -> Result<(), LedgerError> {
        ledger.validate()?;
        let parent = self.path.parent().ok_or(LedgerError::InvalidData)?;
        fs::create_dir_all(parent)?;
        let temp = self.path.with_extension("json.tmp");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp)?;
        file.write_all(&serde_json::to_vec_pretty(ledger)?)?;
        file.sync_all()?;
        fs::rename(temp, &self.path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Currency, Money};
    #[test]
    fn deleting_removes_history_persists_and_preserves_other_wallets() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let mut ledger = Ledger::default();
        let date = "2026-10-05".parse().unwrap();
        let money = |text| Money::parse(text).unwrap();
        let first = ledger
            .create("零钱", Currency::Cny, money("1000"), date, "09:00")
            .unwrap();
        let second = ledger
            .create("储蓄", Currency::Cny, money("500"), date, "09:01")
            .unwrap();
        ledger
            .record(first, money("200"), true, date, "09:02")
            .unwrap();
        ledger
            .record(first, money("900"), false, date, "09:03")
            .unwrap();
        assert!(ledger.delete_wallet(999).is_err());
        assert_eq!(ledger.total_on(date).unwrap(), 140000);
        ledger.delete_wallet(first).unwrap();
        store.save(&ledger).unwrap();
        let mut restored = store.load().unwrap();
        assert!(restored.wallet(first).is_none());
        assert_eq!(restored.wallets().len(), 1);
        assert_eq!(restored.balance(second).cents(), 50000);
        assert!(restored.entries().iter().all(|e| e.wallet_id() == second));
        let summary = restored.summary(date, date).unwrap();
        assert_eq!((summary.income(), summary.expense()), (0, 0));
        restored.delete_wallet(second).unwrap();
        store.save(&restored).unwrap();
        assert!(store.load().unwrap().entries().is_empty());
        let fresh = restored
            .create("新钱包", Currency::Cny, money("0"), date, "10:00")
            .unwrap();
        assert!(fresh > second);
    }
    #[test]
    fn saves_reload_and_corruption_is_not_silently_reset() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let mut ledger = store.load().unwrap();
        let date = "2026-10-04".parse().unwrap();
        ledger
            .create(
                "零钱",
                Currency::Cny,
                Money::parse("123.45").unwrap(),
                date,
                "09:00",
            )
            .unwrap();
        store.save(&ledger).unwrap();
        assert_eq!(store.load().unwrap().total_on(date).unwrap(), 12345);
        fs::write(dir.path().join("wallets-v1.json"), b"broken").unwrap();
        assert!(store.load().is_err());
        assert_eq!(
            fs::read(dir.path().join("wallets-v1.json")).unwrap(),
            b"broken"
        );
    }
}
