# jizhang

Android Rust 动态库。`android.rs` 初始化 gpui-mobile 和 GPUI Kit Root；`ui/` 按钱包、编辑、统计页面划分；`rates.rs` 在后台线程通过 Android HTTPS 获取汇率。依赖 wallet-ledger，界面不自行计算或持久化流水。

通过根目录 `android/gradlew -p android assembleDebug` 构建。宿主机只运行 ledger 测试，不提供桌面入口。平台测试见 [Android 手动测试](../../android/manual-tests.md)。

钱包吉祥物为基于本项目已确认设计图生成的透明 PNG。自定义钱包、统计图标来自 Lucide，许可证见 `assets/LICENSE-LUCIDE`；其他图标通过 gpui-kit assets 编译时嵌入。
