use gpui_kit::{component::Root, *};

gpui_kit::assets::icon_assets!(
    WalletAssets,
    [
        Settings,
        Plus,
        ArrowLeft,
        ChevronLeft,
        ChevronRight,
        Sun,
        Moon,
        RefreshCw,
        Check,
        Copy,
        Search,
        Eye,
        EyeOff,
        LoaderCircle,
        ChevronDown,
        ChevronUp,
        TriangleAlert,
        X,
        Close
    ]
);

#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("jizhang"),
    );
    gpui_mobile::android::jni::install_panic_hook();
    let _platform = gpui_mobile::android::jni::init_platform(&app);
    let platform = gpui_mobile::android::jni::shared_platform().expect("Android platform");
    let data_path = app
        .internal_data_path()
        .expect("private app data directory");
    Application::with_platform(platform.into_rc())
        .with_assets(WalletAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|cx| crate::ui::WalletApp::new(data_path, window, cx));
                view.update(cx, |view, cx| view.sync_rates(false, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("wallet window");
        });
    // gpui-mobile owns process-wide platform state. A destroyed Activity must
    // start a fresh process when Android recreates it, never a second GPUI App.
    std::process::exit(0);
}
