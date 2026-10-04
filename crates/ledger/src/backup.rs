use crate::{Ledger, LedgerError};
use serde::{Deserialize, Serialize};

/// A portable, versioned full ledger snapshot. Import never merges operation IDs.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    format: String,
    version: u32,
    exported_at: String,
    ledger: Ledger,
}
impl Backup {
    pub const MAX_BYTES: usize = 16 * 1024 * 1024;
    pub fn encode(ledger: &Ledger) -> Result<String, LedgerError> {
        ledger.validate()?;
        let backup = Self {
            format: "jizhang-backup".into(),
            version: 1,
            exported_at: chrono::Utc::now().to_rfc3339(),
            ledger: ledger.clone(),
        };
        let json = serde_json::to_string_pretty(&backup)?;
        if json.len() > Self::MAX_BYTES {
            return Err(LedgerError::BackupTooLarge);
        }
        Ok(json)
    }
    pub fn decode(json: &str) -> Result<Self, LedgerError> {
        if json.len() > Self::MAX_BYTES {
            return Err(LedgerError::BackupTooLarge);
        }
        let backup: Self = serde_json::from_str(json).map_err(|_| LedgerError::InvalidBackup)?;
        if backup.format != "jizhang-backup"
            || backup.version != 1
            || chrono::DateTime::parse_from_rfc3339(&backup.exported_at).is_err()
        {
            return Err(LedgerError::InvalidBackup);
        }
        backup.ledger.validate()?;
        Ok(backup)
    }
    pub fn ledger(&self) -> &Ledger {
        &self.ledger
    }
    pub fn into_ledger(self) -> Ledger {
        self.ledger
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Currency, Money, Rate, Store};
    fn sample() -> Ledger {
        let mut ledger = Ledger::default();
        let date = "2026-10-05".parse().unwrap();
        ledger.set_rate(Rate::new(date, date, 7_000_000).unwrap());
        let id = ledger
            .create(
                "旅行 🧳",
                Currency::Usd,
                Money::parse("100").unwrap(),
                date,
                "12:00",
            )
            .unwrap();
        ledger
            .record(id, Money::parse("20").unwrap(), true, date, "12:01")
            .unwrap();
        ledger.set_dark(true);
        ledger
    }
    #[test]
    fn rename_and_full_backup_roundtrip_preserve_history_and_currency() {
        let mut ledger = sample();
        let id = ledger.wallets()[0].id();
        let before = serde_json::to_value(ledger.entries()).unwrap();
        ledger.rename_wallet(id, "  新名字  ").unwrap();
        assert_eq!(ledger.wallet(id).unwrap().name(), "新名字");
        assert!(ledger.rename_wallet(id, "\n").is_err());
        assert!(ledger.rename_wallet(id, "内部\n换行").is_err());
        assert!(ledger.rename_wallet(999, "名称").is_err());
        assert_eq!(serde_json::to_value(ledger.entries()).unwrap(), before);
        let backup = Backup::decode(&Backup::encode(&ledger).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(backup.ledger()).unwrap(),
            serde_json::to_value(&ledger).unwrap()
        );
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let old = Ledger::default();
        store.save(&old).unwrap();
        store.replace_with_backup(backup.ledger()).unwrap();
        assert_eq!(store.load().unwrap().wallets()[0].name(), "新名字");
        assert!(store.load_before_import().unwrap().wallets().is_empty());
        store
            .replace_with_backup(&store.load_before_import().unwrap())
            .unwrap();
        assert!(store.load().unwrap().wallets().is_empty());
        assert_eq!(store.load_before_import().unwrap().wallets().len(), 1);
    }
    #[test]
    fn invalid_backups_never_replace_current_data() {
        let ledger = sample();
        let good: serde_json::Value =
            serde_json::from_str(&Backup::encode(&ledger).unwrap()).unwrap();
        let mutations = [
            ("/version", serde_json::json!(99)),
            ("/ledger/next_id", serde_json::json!(u64::MAX)),
            ("/ledger/entries/1/balance", serde_json::json!(1)),
            ("/ledger/entries/1/time", serde_json::json!("99:99")),
            ("/ledger/entries/1/id", serde_json::json!(2)),
            ("/ledger/rates", serde_json::json!([])),
        ];
        for (path, value) in mutations {
            let mut bad = good.clone();
            *bad.pointer_mut(path).unwrap() = value;
            assert!(Backup::decode(&bad.to_string()).is_err(), "{path}");
        }
        assert!(Backup::decode("{}").is_err());
        assert!(Backup::decode(&" ".repeat(Backup::MAX_BYTES + 1)).is_err());
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        store.save(&ledger).unwrap();
        let before = std::fs::read(dir.path().join("wallets-v1.json")).unwrap();
        let mut bad = serde_json::to_value(&ledger).unwrap();
        bad["entries"][1]["balance"] = serde_json::json!(1);
        let bad: Ledger = serde_json::from_value(bad).unwrap();
        assert!(store.replace_with_backup(&bad).is_err());
        assert_eq!(
            std::fs::read(dir.path().join("wallets-v1.json")).unwrap(),
            before
        );
    }
    #[test]
    fn saving_unchanged_balance_does_not_create_zero_adjustment() {
        let mut ledger = sample();
        let id = ledger.wallets()[0].id();
        let len = ledger.entries().len();
        ledger
            .record(
                id,
                ledger.balance(id),
                false,
                "2026-10-05".parse().unwrap(),
                "12:05",
            )
            .unwrap();
        assert_eq!(ledger.entries().len(), len);
    }
}
