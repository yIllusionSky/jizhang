use super::*;

impl WalletApp {
    pub(super) fn rate_status(&self, cx: &App) -> Div {
        let mut content = column().gap_2();
        if let Some(rate) = self.ledger.latest_rate() {
            content = content
                .child(
                    row()
                        .justify_between()
                        .child("美元兑人民币")
                        .child(value(format!("{:.4}", rate.value()), cx)),
                )
                .child(muted(rate.date().format("%Y/%m/%d").to_string(), cx));
            if rate.synced_on() != Local::now().date_naive() {
                content = content.child(muted("未更新 · 使用上次汇率", cx));
            }
        } else {
            content = content.child(muted("尚未同步", cx));
        }
        if self.sync_error.is_some() {
            content = content.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child("同步失败，请重试"),
            );
        }
        content
    }
    pub(super) fn home(&self, cx: &mut Context<Self>) -> Div {
        let today = Local::now().date_naive();
        let total = self
            .ledger
            .total_on(today)
            .map(|v| Currency::Cny.format(v))
            .unwrap_or_else(|_| "待同步汇率".into());
        let large_total = total.chars().count() > 16;
        let summary = row()
            .justify_between()
            .p_5()
            .rounded(cx.theme().radius_lg)
            .bg(cx.theme().accent)
            .child(
                column()
                    .gap_3()
                    .flex_1()
                    .min_w_0()
                    .child(muted("总资产 · 人民币", cx))
                    .child(
                        value(total, cx)
                            .text_3xl()
                            .when(large_total, |d| d.text_2xl()),
                    ),
            )
            .when(!large_total, |d| {
                d.child(
                    img(self.mascot.clone())
                        .w_16()
                        .h_16()
                        .object_fit(ObjectFit::Contain),
                )
            });
        let mut content = column().gap_3().child(summary);
        if self
            .ledger
            .wallets()
            .iter()
            .any(|w| w.currency() == Currency::Usd)
            && self
                .ledger
                .latest_rate()
                .is_some_and(|r| r.synced_on() != today)
        {
            content = content.child(muted("汇率未更新", cx));
        }
        if self.ledger.wallets().is_empty() {
            content = content.child(
                column()
                    .py_8()
                    .items_center()
                    .child(
                        img(self.mascot.clone())
                            .w_24()
                            .h_24()
                            .object_fit(ObjectFit::Contain),
                    )
                    .child(muted("还没有钱包", cx)),
            );
        }
        for wallet in self.ledger.wallets() {
            let id = wallet.id();
            let updated = self
                .ledger
                .last_entry(id)
                .map(|e| {
                    format!(
                        "{} · {}",
                        wallet.currency().label(),
                        e.date().format("%m/%d")
                    )
                })
                .unwrap_or_default();
            let balance = wallet.currency().format(self.ledger.balance(id).cents());
            let large = balance.chars().count() > 13;
            content = content.child(
                Button::new(("wallet", id))
                    .ghost()
                    .w_full()
                    .h_auto()
                    .p_4()
                    .rounded(cx.theme().radius_lg)
                    .bg(cx.theme().group_box)
                    .child(
                        row()
                            .w_full()
                            .gap_3()
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().accent)
                                    .child(wallet_icon().size_5()),
                            )
                            .child(
                                column()
                                    .gap_1()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .truncate()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(wallet.name().to_owned()),
                                    )
                                    .child(muted(updated, cx)),
                            )
                            .child(
                                value(balance, cx)
                                    .text_lg()
                                    .when(large, |d| d.text_base())
                                    .flex_shrink_0(),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, w, cx| this.open_wallet(id, w, cx))),
            );
        }
        content.child(
            Button::new("add-wallet")
                .ghost()
                .h_12()
                .w_full()
                .icon(IconName::Plus)
                .label("添加钱包")
                .text_color(cx.theme().primary)
                .on_click(cx.listener(|this, _, w, cx| this.open_create(w, cx))),
        )
    }
}
