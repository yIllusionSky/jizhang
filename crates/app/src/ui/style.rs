use gpui_kit::{
    component::{ActiveTheme, Icon, Theme, ThemeMode},
    *,
};

pub fn apply(dark: bool, cx: &mut App) {
    let _ = gpui_mobile::android::jni::with_env(|env| {
        let activity = gpui_mobile::android::jni::activity(env)?;
        env.call_method(
            &activity,
            jni::jni_str!("setWalletTheme"),
            jni::jni_sig!("(Z)V"),
            &[jni::objects::JValue::Bool(dark)],
        )
        .map_err(|error| {
            env.exception_clear();
            error.to_string()
        })?;
        Ok(())
    });
    Theme::change(
        if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );
    Theme::update(cx, |theme| {
        theme.font_family = "Noto Sans CJK SC".into();
        theme.mono_font_family = "Roboto".into();
        // Theme definitions anchor the semantic rem scale and palette.
        theme.font_size = px(16.);
        theme.radius = px(12.);
        theme.radius_lg = px(20.);
        theme.shadow = false;
        let palette = if dark {
            [
                0x17201C, 0xEAF2E9, 0xA6B5AA, 0xA3CD8D, 0x293D2D, 0x35473B, 0x24322A, 0x17201C,
            ]
        } else {
            [
                0xF6F8F5, 0x26332A, 0x5F6D64, 0x427B3B, 0xE5EFDF, 0xD4DDD3, 0xFDFEFC, 0xFDFEFC,
            ]
        };
        let [bg, fg, muted, primary, accent, border, surface, primary_fg] =
            palette.map(|c| rgb(c).into());
        let danger: Hsla = rgb(if dark { 0xF29699 } else { 0xA82E36 }).into();
        theme.colors.danger = danger;
        theme.colors.button_danger = danger;
        theme.colors.button_danger_hover = danger;
        theme.colors.button_danger_active = danger;
        theme.colors.button_danger_foreground = primary_fg;
        theme.colors.background = bg;
        theme.colors.foreground = fg;
        theme.colors.muted_foreground = muted;
        theme.colors.primary = primary;
        theme.colors.primary_foreground = primary_fg;
        theme.colors.primary_hover = primary;
        theme.colors.primary_active = primary;
        theme.colors.accent = accent;
        theme.colors.accent_foreground = primary;
        theme.colors.muted = accent;
        theme.colors.border = border;
        theme.colors.input = border;
        theme.colors.group_box = surface;
        theme.colors.group_box_foreground = fg;
        theme.colors.popover = surface;
        theme.colors.popover_foreground = fg;
        theme.colors.ring = primary;
        theme.colors.button = surface;
        theme.colors.button_foreground = fg;
        theme.colors.button_hover = accent;
        theme.colors.button_active = accent;
        theme.colors.button_primary = primary;
        theme.colors.button_primary_hover = primary;
        theme.colors.button_primary_active = primary;
        theme.colors.button_primary_foreground = primary_fg;
        theme.colors.secondary = accent;
        theme.colors.secondary_foreground = fg;
    });
}

pub fn wallet_icon() -> Icon {
    Icon::default().data(include_bytes!("../../assets/wallet.svg"))
}
pub fn chart_icon() -> Icon {
    Icon::default().data(include_bytes!("../../assets/chart.svg"))
}
pub fn column() -> Div {
    div().flex().flex_col().gap_4()
}
pub fn row() -> Div {
    div().flex().items_center().gap_3()
}
pub fn muted(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text.into())
}
pub fn heading(text: impl Into<SharedString>) -> Div {
    div()
        .text_xl()
        .font_weight(FontWeight::BOLD)
        .child(text.into())
}
pub fn value(text: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .font_family(cx.theme().mono_font_family.clone())
        .font_features(FontFeatures(std::sync::Arc::new(vec![("tnum".into(), 1)])))
        .font_weight(FontWeight::BOLD)
        .child(text.into())
}
