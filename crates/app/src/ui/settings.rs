use super::*;
use wallet_ledger::Backup;

impl WalletApp {
    pub(super) fn settings(&self, cx: &mut Context<Self>) -> Div {
        column()
            .child(heading("外观"))
            .child(
                Button::new("theme-toggle")
                    .h_12()
                    .disabled(self.busy || self.load_failed)
                    .label(if self.ledger.is_dark() {
                        "切换浅色外观"
                    } else {
                        "切换深色外观"
                    })
                    .icon(if self.ledger.is_dark() {
                        IconName::Sun
                    } else {
                        IconName::Moon
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_theme(cx))),
            )
            .child(heading("数据"))
            .child(row().children([true, false].into_iter().map(|export| {
                Button::new(if export { "export" } else { "import" })
                    .h_12()
                    .flex_1()
                    .label(if export {
                        "导出备份"
                    } else {
                        "导入备份"
                    })
                    .disabled(self.busy || (export && self.load_failed))
                    .on_click(cx.listener(move |this, _, _, cx| this.transfer(export, cx)))
            })))
            .when(self.has_restore, |d| {
                d.child(
                    Button::new("restore-import")
                        .ghost()
                        .h_12()
                        .label("恢复导入前账本")
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _, _, cx| this.restore_preview(cx))),
                )
            })
            .when(self.busy, |d| d.child(muted("正在处理…", cx)))
            .when_some(self.message.clone(), |d, message| {
                d.child(muted(message, cx))
            })
            .child(heading("汇率"))
            .child(self.rate_status(cx))
            .child(
                Button::new("refresh-settings")
                    .h_12()
                    .disabled(self.busy || self.syncing || self.load_failed)
                    .label(if self.syncing {
                        "正在同步"
                    } else {
                        "同步汇率"
                    })
                    .icon(IconName::RefreshCw)
                    .on_click(cx.listener(|this, _, _, cx| this.sync_rates(true, cx))),
            )
            .child(muted("汇率来源：Frankfurter / ECB", cx))
    }
    fn transfer(&mut self, export: bool, cx: &mut Context<Self>) {
        if self.busy || (export && self.load_failed) {
            return;
        }
        self.busy = true;
        self.error = None;
        self.message = None;
        let ledger = self.ledger.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let prepared = if export {
                cx.background_executor()
                    .spawn(
                        async move { Backup::encode(&ledger).map(Some).map_err(|e| e.to_string()) },
                    )
                    .await
            } else {
                Ok(None)
            };
            let result = match prepared {
                Ok(snapshot) => crate::documents::choose(snapshot).await,
                Err(error) => Err(error),
            };
            let parsed = match result {
                Ok(Some(json)) if !export => {
                    cx.background_executor()
                        .spawn(async move {
                            Backup::decode(&json)
                                .map(|backup| Some(backup.into_ledger()))
                                .map_err(|e| e.to_string())
                        })
                        .await
                }
                Ok(Some(_)) => {
                    let _ = this.update(cx, |this, _| this.message = Some("备份已导出".into()));
                    Ok(None)
                }
                Ok(None) => Ok(None),
                Err(error) => Err(error),
            };
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match parsed {
                    Ok(Some(ledger)) => {
                        this.pending_import = Some(ledger);
                        this.page = Page::Import;
                    }
                    Ok(None) => {}
                    Err(error) => this.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn restore_preview(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        let store = self.store.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { store.load_before_import() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(ledger) => {
                        this.pending_import = Some(ledger);
                        this.page = Page::Import;
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub(super) fn import_preview(&self, cx: &App) -> Div {
        let Some(ledger) = &self.pending_import else {
            return column();
        };
        column()
            .child(heading(format!(
                "{} 个钱包 · {} 条记录",
                ledger.wallets().len(),
                ledger.entries().len()
            )))
            .children(ledger.wallets().iter().take(5).map(|wallet| {
                row()
                    .justify_between()
                    .child(div().min_w_0().truncate().child(wallet.name().to_owned()))
                    .child(muted(wallet.currency().label(), cx))
            }))
            .when(ledger.wallets().len() > 5, |d| {
                d.child(muted("其余钱包也会一并导入", cx))
            })
            .child(muted("将替换当前账本。导入后可在设置恢复原账本。", cx))
    }
    pub(super) fn import_bottom(&self, cx: &mut Context<Self>) -> Div {
        row()
            .px_6()
            .py_3()
            .flex_shrink_0()
            .child(
                Button::new("cancel-import")
                    .h_12()
                    .flex_1()
                    .label("取消")
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, w, cx| this.navigate(Page::Settings, w, cx))),
            )
            .child(
                Button::new("confirm-import")
                    .primary()
                    .h_12()
                    .flex_1()
                    .label(if self.busy {
                        "正在导入"
                    } else {
                        "替换并导入"
                    })
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, _, cx| this.confirm_import(cx))),
            )
    }
    fn confirm_import(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(ledger) = self.pending_import.clone() else {
            return;
        };
        self.busy = true;
        self.error = None;
        let store = self.store.clone();
        cx.notify();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { store.replace_with_backup(&ledger).map(|_| ledger) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                match result {
                    Ok(ledger) => {
                        this.ledger = ledger;
                        this.pending_import = None;
                        this.load_failed = false;
                        this.has_restore = this.store.has_before_import();
                        this.generation += 1;
                        this.attempted = None;
                        this.sync_error = None;
                        this.page = Page::Home;
                        apply(this.ledger.is_dark(), cx);
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }
}
