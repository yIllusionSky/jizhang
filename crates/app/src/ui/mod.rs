mod editor;
mod home;
mod settings;
mod statistics;
mod style;

use chrono::{Local, NaiveDate};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, IconName, Sizable, WindowExt,
        button::{Button, ButtonVariants},
        input::{InputEvent, InputState},
    },
    *,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use style::*;
use wallet_ledger::{Currency, Ledger, Money, Period, Store};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Statistics,
    Create,
    Edit(u64),
    Settings,
    Rename(u64),
    Import,
}

pub struct WalletApp {
    ledger: Ledger,
    store: Store,
    page: Page,
    name: Entity<InputState>,
    amount: Entity<InputState>,
    currency: Currency,
    income: bool,
    error: Option<String>,
    load_failed: bool,
    busy: bool,
    message: Option<String>,
    pending_import: Option<Ledger>,
    has_restore: bool,
    generation: u64,
    syncing: bool,
    sync_error: Option<String>,
    attempted: Option<NaiveDate>,
    period: Period,
    anchor: NaiveDate,
    mascot: Arc<Image>,
    focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
    _ticker: Task<()>,
}

impl WalletApp {
    pub fn new(path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let store = Store::new(path);
        let has_restore = store.has_before_import();
        let (ledger, error) = match store.load() {
            Ok(ledger) => (ledger, None),
            Err(e) => (Ledger::default(), Some(e.to_string())),
        };
        apply(ledger.is_dark(), cx);
        let name = cx.new(|cx| InputState::new(window, cx).placeholder("例如：日常零钱"));
        let amount = cx.new(|cx| InputState::new(window, cx).placeholder("0.00"));
        let mut subscriptions: Vec<Subscription> = [(&name, false), (&amount, true)]
            .into_iter()
            .map(|(state, decimal)| {
                cx.subscribe_in(state, window, move |this, _, event, _, cx| {
                    if matches!(event, InputEvent::Focus) {
                        crate::documents::set_input_type(decimal);
                    }
                    if matches!(event, InputEvent::Focus | InputEvent::Blur) {
                        // Android updates content insets after its resize callback.
                        // Refresh once after the IME animation, without continuous polling.
                        cx.spawn(async move |this, cx| {
                            cx.background_executor()
                                .timer(Duration::from_millis(400))
                                .await;
                            let _ = this.update(cx, |_, cx| cx.notify());
                        })
                        .detach();
                    }
                    if matches!(event, InputEvent::Change) {
                        let had_error = this.error.take().is_some();
                        // Name edits already redraw their own Input entity.
                        if decimal || had_error {
                            cx.notify();
                        }
                    }
                })
            })
            .collect();
        subscriptions.push(cx.observe_window_bounds(window, |_, window, cx| {
            // Mobile publishes content insets just after its resize callback.
            let view = cx.entity();
            window.on_next_frame(move |_, cx| cx.notify(view.entity_id()));
        }));
        let ticker = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(60))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        this.sync_rates(false, cx);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            ledger,
            store,
            page: Page::Home,
            name,
            amount,
            currency: Currency::Cny,
            income: false,
            load_failed: error.is_some(),
            busy: false,
            message: None,
            pending_import: None,
            has_restore,
            generation: 0,
            error,
            syncing: false,
            sync_error: None,
            attempted: None,
            period: Period::Month,
            anchor: Local::now().date_naive(),
            mascot: Arc::new(Image::from_bytes(
                ImageFormat::Webp,
                crate::android::load_mascot(),
            )),
            focus: cx.focus_handle(),
            _subscriptions: subscriptions,
            _ticker: ticker,
        }
    }
    pub fn sync_rates(&mut self, force: bool, cx: &mut Context<Self>) {
        let today = Local::now().date_naive();
        if self.syncing
            || self.busy
            || self.load_failed
            || (!force
                && (self.attempted == Some(today)
                    || self
                        .ledger
                        .latest_rate()
                        .is_some_and(|r| r.synced_on() == today)))
        {
            return;
        }
        self.syncing = true;
        self.attempted = Some(today);
        self.sync_error = None;
        cx.notify();
        let generation = self.generation;
        let job = cx
            .background_executor()
            .spawn(async { crate::rates::fetch() });
        cx.spawn(async move |this, cx| {
            let result = job.await;
            let _ = this.update(cx, |this, cx| {
                this.syncing = false;
                if this.generation != generation || this.busy {
                    this.attempted = None;
                    cx.notify();
                    return;
                }
                match result {
                    Ok(rate) => {
                        let mut next = this.ledger.clone();
                        next.set_rate(rate);
                        match this.store.save(&next) {
                            Ok(()) => this.ledger = next,
                            Err(e) => this.sync_error = Some(e.to_string()),
                        }
                    }
                    Err(e) => this.sync_error = Some(e),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn navigate(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.page = page;
        self.message = None;
        if page != Page::Import {
            self.pending_import = None;
        }
        if !self.load_failed {
            self.error = None;
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn open_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.currency = Currency::Cny;
        self.income = false;
        self.name.update(cx, |s, cx| s.set_value("", window, cx));
        self.amount.update(cx, |s, cx| s.set_value("", window, cx));
        self.navigate(Page::Create, window, cx);
    }
    fn open_wallet(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        self.income = false;
        self.currency = self.ledger.wallet(id).unwrap().currency();
        let balance = self.ledger.balance(id).input();
        self.amount
            .update(cx, |s, cx| s.set_value(balance, window, cx));
        self.navigate(Page::Edit(id), window, cx);
    }
    fn open_rename(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(wallet) = self.ledger.wallet(id) {
            let name = wallet.name().to_owned();
            self.name
                .update(cx, |state, cx| state.set_value(name, window, cx));
            self.navigate(Page::Rename(id), window, cx);
            let name = self.name.clone();
            window.on_next_frame(move |window, cx| {
                crate::documents::set_input_type(false);
                name.update(cx, |state, cx| state.focus(window, cx));
                crate::documents::show_keyboard();
            });
        }
    }
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.load_failed || self.busy {
            return;
        }
        let mut next = self.ledger.clone();
        let now = Local::now();
        let result = if let Page::Rename(id) = self.page {
            next.rename_wallet(id, &self.name.read(cx).value())
                .and_then(|_| self.store.save(&next))
        } else {
            Money::parse(&self.amount.read(cx).value()).and_then(|amount| {
                match self.page {
                    Page::Create => next
                        .create(
                            &self.name.read(cx).value(),
                            self.currency,
                            amount,
                            now.date_naive(),
                            &now.format("%H:%M").to_string(),
                        )
                        .map(|_| ()),
                    Page::Edit(id) => next.record(
                        id,
                        amount,
                        self.income,
                        now.date_naive(),
                        &now.format("%H:%M").to_string(),
                    ),
                    _ => return Ok(()),
                }?;
                self.store.save(&next)
            })
        };
        match result {
            Ok(()) => {
                self.ledger = next;
                let page = if let Page::Rename(id) = self.page {
                    Page::Edit(id)
                } else {
                    Page::Home
                };
                self.navigate(page, window, cx);
            }
            Err(e) => {
                self.error = Some(e.to_string());
                cx.notify();
            }
        }
    }
    fn confirm_delete(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        let Some(wallet) = self.ledger.wallet(id) else {
            return;
        };
        let name = wallet.name().to_owned();
        let view = cx.entity().downgrade();
        window.focus(&self.focus, cx);
        let width = window.viewport_size().width - rems(3.).to_pixels(window.rem_size());
        window.open_dialog(cx, move |dialog, _, cx| {
            let view = view.clone();
            dialog
                .w(width)
                .title("删除钱包？")
                .close_button(false)
                .child(
                    column()
                        .gap_2()
                        .child(div().child(format!("“{name}”")))
                        .child(muted("历史记录也会删除，无法恢复。", cx)),
                )
                .footer(
                    row()
                        .w_full()
                        .child(
                            Button::new("cancel-delete")
                                .h_12()
                                .flex_1()
                                .label("取消")
                                .on_click(|_, w, cx| w.close_dialog(cx)),
                        )
                        .child(
                            Button::new("confirm-delete")
                                .danger()
                                .h_12()
                                .flex_1()
                                .label("删除")
                                .on_click(move |_, w, cx| {
                                    w.close_dialog(cx);
                                    let _ = view.update(cx, |this, cx| {
                                        let mut next = this.ledger.clone();
                                        match next
                                            .delete_wallet(id)
                                            .and_then(|_| this.store.save(&next))
                                        {
                                            Ok(()) => {
                                                this.ledger = next;
                                                this.navigate(Page::Home, w, cx);
                                            }
                                            Err(error) => {
                                                this.error = Some(error.to_string());
                                                cx.notify();
                                            }
                                        }
                                    });
                                }),
                        ),
                )
        });
    }
    fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        if self.load_failed || self.busy {
            return;
        }
        let mut next = self.ledger.clone();
        next.set_dark(!next.is_dark());
        match self.store.save(&next) {
            Ok(()) => {
                self.ledger = next;
                apply(self.ledger.is_dark(), cx);
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        cx.notify();
    }
    fn header(&self, cx: &mut Context<Self>) -> Div {
        let title = match self.page {
            Page::Home => "我的钱包".to_owned(),
            Page::Statistics => "统计".into(),
            Page::Create => "新建钱包".into(),
            Page::Settings => "设置".into(),
            Page::Rename(_) => "钱包改名".into(),
            Page::Import => "导入账本".into(),
            Page::Edit(id) => self
                .ledger
                .wallet(id)
                .map(|w| w.name().to_owned())
                .unwrap_or_default(),
        };
        row()
            .justify_between()
            .px_6()
            .py_4()
            .flex_shrink_0()
            .child(
                row()
                    .min_w_0()
                    .flex_1()
                    .when(
                        matches!(
                            self.page,
                            Page::Create
                                | Page::Edit(_)
                                | Page::Settings
                                | Page::Rename(_)
                                | Page::Import
                        ),
                        |r| {
                            r.child(
                                Button::new("back")
                                    .ghost()
                                    .large()
                                    .h_12()
                                    .w_12()
                                    .icon(IconName::ArrowLeft)
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        let page = match this.page {
                                            Page::Rename(id) => Page::Edit(id),
                                            Page::Import => Page::Settings,
                                            _ => Page::Home,
                                        };
                                        this.navigate(page, window, cx)
                                    })),
                            )
                        },
                    )
                    .child(
                        div()
                            .min_w_0()
                            .text_2xl()
                            .truncate()
                            .font_weight(FontWeight::BOLD)
                            .child(title),
                    ),
            )
            .when_some(
                if let Page::Edit(id) = self.page {
                    Some(id)
                } else {
                    None
                },
                |r, id| {
                    r.child(
                        Button::new("rename-wallet")
                            .ghost()
                            .h_12()
                            .label("改名")
                            .on_click(
                                cx.listener(move |this, _, w, cx| this.open_rename(id, w, cx)),
                            ),
                    )
                    .child(
                        Button::new("delete-wallet")
                            .ghost()
                            .h_12()
                            .label("删除")
                            .text_color(cx.theme().danger)
                            .on_click(
                                cx.listener(move |this, _, w, cx| this.confirm_delete(id, w, cx)),
                            ),
                    )
                },
            )
            .when(self.page == Page::Home, |r| {
                r.child(
                    Button::new("settings")
                        .ghost()
                        .large()
                        .h_12()
                        .w_12()
                        .icon(IconName::Settings)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.navigate(Page::Settings, window, cx)
                        })),
                )
            })
    }
    fn bottom(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.page == Page::Import {
            return self.import_bottom(cx).into_any_element();
        }
        if matches!(self.page, Page::Create | Page::Edit(_) | Page::Rename(_)) {
            let label = if matches!(self.page, Page::Rename(_)) {
                "保存名称"
            } else if self.page == Page::Create {
                "创建钱包"
            } else if self.income {
                "记录收入"
            } else {
                "保存余额"
            };
            return div()
                .px_6()
                .py_3()
                .flex_shrink_0()
                .child(
                    Button::new("save")
                        .primary()
                        .large()
                        .h_12()
                        .w_full()
                        .label(label)
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                )
                .into_any_element();
        }
        if self.page == Page::Settings {
            return div().into_any_element();
        }
        row()
            .px_6()
            .py_3()
            .border_t_1()
            .border_color(cx.theme().border)
            .flex_shrink_0()
            .child(
                Button::new("wallets-tab")
                    .ghost()
                    .large()
                    .h_12()
                    .flex_1()
                    .icon(wallet_icon())
                    .label("钱包")
                    .when(self.page == Page::Home, |b| {
                        b.bg(cx.theme().accent).text_color(cx.theme().primary)
                    })
                    .on_click(cx.listener(|this, _, w, cx| this.navigate(Page::Home, w, cx))),
            )
            .child(
                Button::new("stats-tab")
                    .ghost()
                    .large()
                    .h_12()
                    .flex_1()
                    .icon(chart_icon())
                    .label("统计")
                    .when(self.page == Page::Statistics, |b| {
                        b.bg(cx.theme().accent).text_color(cx.theme().primary)
                    })
                    .on_click(cx.listener(|this, _, w, cx| this.navigate(Page::Statistics, w, cx))),
            )
            .into_any_element()
    }
}

impl Render for WalletApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // NativeActivity reports the surface's actual safe/keyboard insets.
        let insets = gpui_mobile::android::jni::platform()
            .and_then(|p| p.primary_window())
            .map(|w| w.safe_area_insets_logical())
            .unwrap_or_default();
        let body = if self.load_failed && !matches!(self.page, Page::Settings | Page::Import) {
            column()
                .child(heading("暂时无法读取钱包"))
                .child(muted("原始数据已保留。请关闭应用后重试。", cx))
        } else {
            match self.page {
                Page::Home => self.home(cx),
                Page::Create | Page::Edit(_) | Page::Rename(_) => self.editor(cx),
                Page::Import => self.import_preview(cx),
                Page::Statistics => self.statistics(cx),
                Page::Settings => self.settings(cx),
            }
        };
        let _ = window;
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .font_family(cx.theme().font_family.clone())
            .text_base()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    if this.name.read(cx).focus_handle(cx).is_focused(window)
                        || this.amount.read(cx).focus_handle(cx).is_focused(window)
                    {
                        window.focus(&this.focus, cx);
                    } else if this.page != Page::Home {
                        let page = match this.page {
                            Page::Rename(id) => Page::Edit(id),
                            Page::Import => Page::Settings,
                            _ => Page::Home,
                        };
                        this.navigate(page, window, cx);
                    }
                    cx.stop_propagation();
                }
            }))
            // Insets are platform-reported physical geometry, not product spacing.
            .pt(px(insets.top))
            .pb(px(insets.bottom))
            .pl(px(insets.left))
            .pr(px(insets.right))
            .child(self.header(cx))
            .child(
                div()
                    .id("page-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(column().px_6().pb_6().child(body).when_some(
                        self.error.clone().filter(|_| {
                            !matches!(self.page, Page::Create | Page::Edit(_) | Page::Rename(_))
                        }),
                        |d, error| {
                            d.child(div().text_sm().text_color(cx.theme().danger).child(error))
                        },
                    )),
            )
            .when(!self.load_failed || self.page == Page::Import, |d| {
                d.child(self.bottom(cx))
            })
    }
}
