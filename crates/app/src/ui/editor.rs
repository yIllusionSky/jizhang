use super::*;
use gpui_kit::component::{Disableable, input::Input};

impl WalletApp {
    fn mobile_input(&self, decimal: bool, _cx: &mut Context<Self>) -> Div {
        let state = if decimal { &self.amount } else { &self.name };
        let clear_state = state.clone();
        div()
            .capture_any_mouse_down(move |_, _, _| crate::documents::set_input_type(decimal))
            .on_mouse_down(MouseButton::Left, |_, _, _| {
                crate::documents::show_keyboard()
            })
            .child(
                Input::new(state)
                    .large()
                    .h_12()
                    .when(decimal, |input| {
                        input.h_16().text_2xl().prefix(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .child(self.currency.symbol()),
                        )
                    })
                    .suffix(
                        Button::new(if decimal {
                            "clear-amount"
                        } else {
                            "clear-name"
                        })
                        .ghost()
                        .icon(IconName::Close)
                        .h_12()
                        .w_12()
                        .tab_stop(false)
                        .accessibility_label("清空输入")
                        .on_click(move |_, window, cx| {
                            clear_state.update(cx, |state, cx| {
                                state.set_value("", window, cx);
                                state.focus(window, cx);
                            });
                            crate::documents::show_keyboard();
                        }),
                    ),
            )
    }

    pub(super) fn editor(&self, cx: &mut Context<Self>) -> Div {
        if matches!(self.page, Page::Rename(_)) {
            return column()
                .gap_2()
                .child("钱包名称")
                .child(self.mobile_input(false, cx))
                .when_some(self.error.clone(), |d, error| {
                    d.child(div().text_sm().text_color(cx.theme().danger).child(error))
                });
        }
        let create = self.page == Page::Create;
        let mut content = column().gap_6();
        if create {
            content = content
                .child(
                    column()
                        .gap_2()
                        .child("钱包名称")
                        .child(self.mobile_input(false, cx)),
                )
                .child(column().gap_2().child("币种").child(row().gap_2().children(
                    [Currency::Cny, Currency::Usd].into_iter().map(|currency| {
                        Button::new(if currency == Currency::Cny {
                            "cny"
                        } else {
                            "usd"
                        })
                        .ghost()
                        .large()
                        .h_12()
                        .flex_1()
                        .label(if currency == Currency::Cny {
                            "人民币 CNY"
                        } else {
                            "美元 USD"
                        })
                        .when(self.currency == currency, |b| {
                            b.bg(cx.theme().accent).text_color(cx.theme().primary)
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.currency = currency;
                            cx.notify();
                        }))
                    }),
                )));
        } else if let Page::Edit(id) = self.page {
            content = content
                .child(column().gap_2().child(muted("当前余额", cx)).child(
                    value(self.currency.format(self.ledger.balance(id).cents()), cx).text_3xl(),
                ))
                .child(
                    row()
                        .gap_1()
                        .p_1()
                        .bg(cx.theme().group_box)
                        .rounded(cx.theme().radius)
                        .children([false, true].into_iter().map(|income| {
                            Button::new(if income {
                                "income-mode"
                            } else {
                                "balance-mode"
                            })
                            .ghost()
                            .large()
                            .h_12()
                            .flex_1()
                            .label(if income { "记收入" } else { "设置余额" })
                            .when(self.income == income, |b| {
                                b.bg(cx.theme().accent).text_color(cx.theme().primary)
                            })
                            .on_click(cx.listener(
                                move |this, _, w, cx| {
                                    if this.income == income {
                                        return;
                                    }
                                    this.income = income;
                                    this.error = None;
                                    let value = if income {
                                        String::new()
                                    } else {
                                        this.ledger.balance(id).input()
                                    };
                                    this.amount.update(cx, |s, cx| s.set_value(value, w, cx));
                                    cx.notify();
                                },
                            ))
                        })),
                );
        }
        content = content.child(
            column()
                .gap_2()
                .child(if create {
                    "初始余额"
                } else if self.income {
                    "收入金额"
                } else {
                    "设置余额"
                })
                .child(self.mobile_input(true, cx)),
        );
        content = content.when_some(self.error.clone(), |d, error| {
            d.child(div().text_sm().text_color(cx.theme().danger).child(error))
        });
        if create {
            if self.currency == Currency::Usd && self.ledger.latest_rate().is_none() {
                content = content.child(muted("请先同步汇率", cx)).child(
                    Button::new("fetch-for-usd")
                        .h_12()
                        .label("同步汇率")
                        .disabled(self.syncing)
                        .on_click(cx.listener(|this, _, _, cx| this.sync_rates(true, cx))),
                );
            }
        } else if let Page::Edit(id) = self.page {
            if let Ok(amount) = Money::parse(&self.amount.read(cx).value()) {
                let old = self.ledger.balance(id).cents();
                let entered = amount.cents();
                let (after, label, delta) = if self.income {
                    (old + entered, "收入", entered)
                } else if entered < old {
                    (entered, "支出", old - entered)
                } else {
                    (entered, "余额校正", entered - old)
                };
                if delta > 0 {
                    content = content.child(
                        column()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .bg(cx.theme().accent)
                            .child(
                                row()
                                    .justify_between()
                                    .child("记录后余额")
                                    .child(value(self.currency.format(after), cx)),
                            )
                            .child(
                                row()
                                    .justify_between()
                                    .child(label)
                                    .child(value(self.currency.format(delta), cx)),
                            ),
                    );
                }
            }
            let history = self
                .ledger
                .entries()
                .iter()
                .rev()
                .filter(|e| e.wallet_id() == id)
                .take(5);
            content = content.child(heading("最近记录"));
            for entry in history {
                content = content.child(
                    row()
                        .justify_between()
                        .child(column().gap_1().child(entry.kind().label()).child(muted(
                            format!("{} {}", entry.date().format("%m/%d"), entry.time()),
                            cx,
                        )))
                        .child(value(self.currency.format(entry.amount().cents()), cx)),
                );
            }
        }
        content
    }
}
