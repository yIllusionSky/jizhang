use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error("请输入有效金额，最多两位小数")]
    InvalidAmount,
    #[error("金额超出范围（最多 999,999,999.99）")]
    Overflow,
    #[error("收入金额必须大于 0")]
    EmptyIncome,
    #[error("钱包名称需为 1 至 40 个字")]
    InvalidName,
    #[error("钱包不存在")]
    MissingWallet,
    #[error("系统日期早于已有记录，请检查手机日期")]
    ClockReversed,
    #[error("汇率尚未就绪，请联网同步后重试")]
    MissingRate,
    #[error("不是有效的小钱包备份，或版本不支持")]
    InvalidBackup,
    #[error("备份文件过大（最多 16 MB）")]
    BackupTooLarge,
    #[error("数据文件校验失败，原文件已保留")]
    InvalidData,
    #[error("无法读写本机数据：{0}")]
    Storage(#[from] std::io::Error),
    #[error("无法读取数据文件：{0}")]
    Json(#[from] serde_json::Error),
}

/// Non-negative currency amount in cents. Never parsed through floating point.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money(i64);

impl Money {
    pub const MAX: i64 = 99_999_999_999;
    pub fn from_cents(cents: i64) -> Result<Self, LedgerError> {
        if !(0..=Self::MAX).contains(&cents) {
            return Err(LedgerError::Overflow);
        }
        Ok(Self(cents))
    }
    pub fn cents(self) -> i64 {
        self.0
    }
    pub fn parse(text: &str) -> Result<Self, LedgerError> {
        let text = text.trim();
        let mut pieces = text.split('.');
        let whole = pieces.next().unwrap_or_default();
        let fraction = pieces.next().unwrap_or_default();
        if (whole.is_empty() && fraction.is_empty())
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
            || fraction.len() > 2
            || pieces.next().is_some()
        {
            return Err(LedgerError::InvalidAmount);
        }
        let whole: i64 = if whole.is_empty() {
            0
        } else {
            whole.parse().map_err(|_| LedgerError::Overflow)?
        };
        let fraction: i64 = match fraction.len() {
            0 => 0,
            1 => fraction.parse::<i64>().unwrap() * 10,
            _ => fraction.parse().unwrap(),
        };
        let cents = whole
            .checked_mul(100)
            .and_then(|v| v.checked_add(fraction))
            .ok_or(LedgerError::Overflow)?;
        Self::from_cents(cents)
    }
    pub fn input(self) -> String {
        format!("{}.{:02}", self.0 / 100, self.0 % 100)
    }
    pub fn format(cents: i64) -> String {
        let absolute = cents.unsigned_abs();
        let whole = (absolute / 100).to_string();
        let mut text = String::new();
        for (ix, ch) in whole.chars().enumerate() {
            if ix > 0 && (whole.len() - ix).is_multiple_of(3) {
                text.push(',');
            }
            text.push(ch);
        }
        format!(
            "{}{}.{:02}",
            if cents < 0 { "-" } else { "" },
            text,
            absolute % 100
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
    #[default]
    Cny,
    Usd,
}
impl Currency {
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Cny => "¥",
            Self::Usd => "$",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Cny => "人民币",
            Self::Usd => "美元",
        }
    }
    pub fn format(self, cents: i64) -> String {
        format!("{} {}", self.symbol(), Money::format(cents))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decimal_parsing_is_exact_and_rejects_bad_input() {
        assert_eq!(Money::parse("0.29").unwrap().cents(), 29);
        assert_eq!(Money::parse(".5").unwrap().cents(), 50);
        assert_eq!(Money::parse("12.3").unwrap().cents(), 1230);
        for value in [
            "",
            "-1",
            "NaN",
            ".",
            "1e3",
            "1.001",
            "1,000",
            "1.2.3",
            "1000000000",
        ] {
            assert!(Money::parse(value).is_err(), "{value}");
        }
        assert_eq!(Money::format(-123456789), "-1,234,567.89");
    }
}
