use super::*;
use gpui_kit::component::{
    Disableable,
    chart::{BarChart, LineChart},
};

impl WalletApp {
    pub(super) fn statistics(&self, cx: &mut Context<Self>) -> Div {
        let today = Local::now().date_naive();
        let first = self.ledger.first_date().unwrap_or(today);
        let (start, end) = self.period.bounds(self.anchor, first, today);
        let date_label = match self.period {
            Period::Day => self.anchor.format("%Y 年 %m 月 %d 日").to_string(),
            Period::Month => self.anchor.format("%Y 年 %m 月").to_string(),
            Period::Year => self.anchor.format("%Y 年").to_string(),
            Period::All => "全部记录".into(),
        };
        let mut content = column()
            .gap_5()
            .child(
                row()
                    .gap_1()
                    .p_1()
                    .bg(cx.theme().group_box)
                    .rounded(cx.theme().radius)
                    .children(
                        [Period::Day, Period::Month, Period::Year, Period::All]
                            .into_iter()
                            .map(|period| {
                                Button::new(period.label())
                                    .ghost()
                                    .large()
                                    .h_12()
                                    .flex_1()
                                    .label(period.label())
                                    .when(period == self.period, |b| {
                                        b.bg(cx.theme().accent).text_color(cx.theme().primary)
                                    })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.period = period;
                                        this.anchor = Local::now().date_naive();
                                        cx.notify();
                                    }))
                            }),
                    ),
            )
            .child(
                row()
                    .justify_between()
                    .child(
                        Button::new("previous-period")
                            .ghost()
                            .h_12()
                            .w_12()
                            .icon(IconName::ChevronLeft)
                            .disabled(self.period == Period::All || start <= first)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.anchor = this.period.shift(this.anchor, false);
                                cx.notify();
                            })),
                    )
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(date_label))
                    .child(
                        Button::new("next-period")
                            .ghost()
                            .h_12()
                            .w_12()
                            .icon(IconName::ChevronRight)
                            .disabled(self.period == Period::All || end >= today)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.anchor = this
                                    .period
                                    .shift(this.anchor, true)
                                    .min(Local::now().date_naive());
                                cx.notify();
                            })),
                    ),
            );
        if self.ledger.entries().is_empty() {
            return content.child(
                column()
                    .items_center()
                    .py_8()
                    .child(chart_icon().size_12().text_color(cx.theme().primary))
                    .child(heading("还没有记录")),
            );
        }
        let summary = match self.ledger.summary(start, end) {
            Ok(summary) => summary,
            Err(e) => return content.child(muted(e.to_string(), cx)),
        };
        let metrics = [
            ("收入", summary.income()),
            ("支出", summary.expense()),
            ("收支结余", summary.net()),
        ];
        content = content
            .child(
                column()
                    .gap_3()
                    .children(metrics.into_iter().map(|(label, cents)| {
                        row()
                            .justify_between()
                            .child(muted(label, cx))
                            .child(value(Currency::Cny.format(cents), cx).text_xl())
                    })),
            )
            .child(
                row()
                    .justify_between()
                    .child(heading("资产趋势"))
                    .child(value(Currency::Cny.format(summary.end()), cx)),
            );
        let points: Vec<(String, f64)> = self
            .ledger
            .trend(start.max(first), end)
            .unwrap_or_default()
            .into_iter()
            .map(|(date, cents)| (date.format("%m/%d").to_string(), cents as f64 / 100.))
            .collect();
        let primary = cx.theme().primary;
        if points.len() > 1 {
            content = content.child(
                div().h_40().child(
                    LineChart::new(points)
                        .x(|p: &(String, f64)| p.0.clone())
                        .y(|p| p.1)
                        .stroke(primary)
                        .linear()
                        .dot()
                        .y_axis(true)
                        .y_tick_count(3)
                        .x_tick_count(4)
                        .y_tick_format(|v| {
                            if v.abs() >= 10000. {
                                format!("{:.1}万", v / 10000.)
                            } else {
                                format!("{v:.0}")
                            }
                        }),
                ),
            );
        } else {
            content = content.child(
                column()
                    .items_center()
                    .py_5()
                    .bg(cx.theme().accent)
                    .rounded(cx.theme().radius)
                    .child(value(Currency::Cny.format(summary.end()), cx).text_2xl())
                    .child(muted("当天最终余额", cx)),
            );
        }
        let gray = cx.theme().muted_foreground;
        content = content
            .child(heading("收支对比"))
            .child(
                div().h_32().child(
                    BarChart::new(vec![
                        ("收入", summary.income() as f64 / 100.),
                        ("支出", summary.expense() as f64 / 100.),
                    ])
                    .band(|p: &(&str, f64)| SharedString::from(p.0))
                    .value(|p| p.1)
                    .fill(move |p, _, _, _| if p.0 == "收入" { primary } else { gray }),
                ),
            )
            .child(
                row()
                    .justify_between()
                    .child(muted("余额校正", cx))
                    .child(value(Currency::Cny.format(summary.adjustment()), cx)),
            )
            .when(summary.opening() != 0, |d| {
                d.child(
                    row()
                        .justify_between()
                        .child(muted("新增初始资产", cx))
                        .child(value(Currency::Cny.format(summary.opening()), cx)),
                )
            })
            .child(
                row()
                    .justify_between()
                    .child(muted("汇率影响", cx))
                    .child(value(Currency::Cny.format(summary.exchange_effect()), cx)),
            )
            .child(
                row()
                    .justify_between()
                    .child(muted("资产变化", cx))
                    .child(value(Currency::Cny.format(summary.change()), cx)),
            );
        content
    }
}
