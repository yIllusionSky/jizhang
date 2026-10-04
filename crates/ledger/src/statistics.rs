use crate::{EntryKind, Ledger, LedgerError};
use chrono::{Datelike, Duration, Months, NaiveDate};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Period {
    Day,
    #[default]
    Month,
    Year,
    All,
}
impl Period {
    pub fn label(self) -> &'static str {
        match self {
            Self::Day => "日",
            Self::Month => "月",
            Self::Year => "年",
            Self::All => "总计",
        }
    }
    pub fn bounds(
        self,
        anchor: NaiveDate,
        first: NaiveDate,
        today: NaiveDate,
    ) -> (NaiveDate, NaiveDate) {
        let (start, end) = match self {
            Self::Day => (anchor, anchor),
            Self::Month => {
                let start = anchor.with_day(1).unwrap();
                (
                    start,
                    start.checked_add_months(Months::new(1)).unwrap() - Duration::days(1),
                )
            }
            Self::Year => (
                NaiveDate::from_ymd_opt(anchor.year(), 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(anchor.year(), 12, 31).unwrap(),
            ),
            Self::All => (first.min(today), today),
        };
        (start, end.min(today))
    }
    pub fn shift(self, anchor: NaiveDate, next: bool) -> NaiveDate {
        match self {
            Self::Day => anchor + Duration::days(if next { 1 } else { -1 }),
            Self::Month | Self::Year => {
                let months = Months::new(if self == Self::Year { 12 } else { 1 });
                if next {
                    anchor.checked_add_months(months)
                } else {
                    anchor.checked_sub_months(months)
                }
                .unwrap_or(anchor)
            }
            Self::All => anchor,
        }
    }
}

#[derive(Debug, Default)]
pub struct Summary {
    income: i64,
    expense: i64,
    adjustment: i64,
    opening: i64,
    start: i64,
    end: i64,
}
impl Summary {
    pub fn income(&self) -> i64 {
        self.income
    }
    pub fn expense(&self) -> i64 {
        self.expense
    }
    pub fn net(&self) -> i64 {
        self.income - self.expense
    }
    pub fn adjustment(&self) -> i64 {
        self.adjustment
    }
    pub fn opening(&self) -> i64 {
        self.opening
    }
    pub fn end(&self) -> i64 {
        self.end
    }
    pub fn change(&self) -> i64 {
        self.end - self.start
    }
    pub fn exchange_effect(&self) -> i64 {
        self.change() - self.net() - self.adjustment - self.opening
    }
}
impl Ledger {
    pub fn summary(&self, start: NaiveDate, end: NaiveDate) -> Result<Summary, LedgerError> {
        let mut summary = Summary {
            start: self.total_on(start.pred_opt().unwrap_or(start))?,
            end: self.total_on(end)?,
            ..Summary::default()
        };
        for entry in self
            .entries()
            .iter()
            .filter(|e| e.date() >= start && e.date() <= end)
        {
            let total = match entry.kind() {
                EntryKind::Opening => &mut summary.opening,
                EntryKind::Income => &mut summary.income,
                EntryKind::Expense => &mut summary.expense,
                EntryKind::Adjustment => &mut summary.adjustment,
            };
            *total = total
                .checked_add(entry.cny_amount())
                .ok_or(LedgerError::Overflow)?;
        }
        Ok(summary)
    }
    /// At most 32 evenly spaced end-of-day points, including both boundaries.
    pub fn trend(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<(NaiveDate, i64)>, LedgerError> {
        let days = (end - start).num_days().max(0);
        let steps = days.min(31);
        (0..=steps)
            .map(|ix| {
                let date = start + Duration::days(if steps == 0 { 0 } else { ix * days / steps });
                Ok((date, self.total_on(date)?))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Currency, Money, Rate};
    fn day(n: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, n).unwrap()
    }
    fn money(text: &str) -> Money {
        Money::parse(text).unwrap()
    }
    #[test]
    fn repeated_same_day_operations_preserve_flows_and_last_balance() {
        let mut ledger = Ledger::default();
        let id = ledger
            .create("零钱", Currency::Cny, money("1000"), day(1), "09:00")
            .unwrap();
        ledger
            .record(id, money("200"), true, day(1), "10:00")
            .unwrap();
        ledger
            .record(id, money("900"), false, day(1), "11:00")
            .unwrap();
        let summary = ledger.summary(day(1), day(1)).unwrap();
        assert_eq!(
            (summary.income(), summary.expense(), summary.end()),
            (20_000, 30_000, 90_000)
        );
        assert_eq!(ledger.total_on(day(3)).unwrap(), 90_000);
        assert_eq!(summary.exchange_effect(), 0);
        ledger.validate().unwrap();
    }
    #[test]
    fn increase_is_adjustment_and_opening_is_not_income() {
        let mut ledger = Ledger::default();
        let id = ledger
            .create("储蓄", Currency::Cny, money("1000"), day(1), "09:00")
            .unwrap();
        ledger
            .record(id, money("1200"), false, day(2), "09:00")
            .unwrap();
        let summary = ledger.summary(day(1), day(4)).unwrap();
        assert_eq!(
            (summary.income(), summary.adjustment(), summary.opening()),
            (0, 20_000, 100_000)
        );
        assert_eq!(summary.exchange_effect(), 0);
    }
    #[test]
    fn fx_revaluation_does_not_rewrite_historical_income() {
        let mut ledger = Ledger::default();
        assert!(
            ledger
                .create("美元", Currency::Usd, money("100"), day(1), "09:00")
                .is_err()
        );
        ledger.set_rate(Rate::new(day(1), day(1), 7_000_000).unwrap());
        let id = ledger
            .create("美元", Currency::Usd, money("100"), day(1), "09:00")
            .unwrap();
        ledger
            .record(id, money("10"), true, day(1), "10:00")
            .unwrap();
        ledger.set_rate(Rate::new(day(2), day(2), 7_100_000).unwrap());
        let summary = ledger.summary(day(1), day(2)).unwrap();
        assert_eq!(
            (summary.income(), summary.end(), summary.exchange_effect()),
            (7000, 78100, 1100)
        );
        assert_eq!(ledger.total_on(day(1)).unwrap(), 77000);
    }
    #[test]
    fn boundaries_and_failed_operations_are_safe() {
        let leap = NaiveDate::from_ymd_opt(2024, 2, 20).unwrap();
        assert_eq!(Period::Month.bounds(leap, leap, day(4)).1.day(), 29);
        let mut ledger = Ledger::default();
        let id = ledger
            .create(
                "零钱",
                Currency::Cny,
                money("999999999.99"),
                day(2),
                "09:00",
            )
            .unwrap();
        assert!(
            ledger
                .record(id, money("1"), true, day(2), "09:01")
                .is_err()
        );
        assert!(
            ledger
                .record(id, money("0"), true, day(2), "09:01")
                .is_err()
        );
        assert!(
            ledger
                .record(id, money("1"), false, day(1), "09:01")
                .is_err()
        );
        assert_eq!(ledger.entries().len(), 1);
    }
}
