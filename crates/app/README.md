# jizhang

Android Rust 动态库。`android.rs` 初始化 gpui-mobile 和 GPUI Kit Root；`ui/` 按钱包、编辑、统计、设置页面划分；`rates.rs` 在后台线程通过 Android HTTPS 获取汇率。`documents.rs` 桥接系统文件选择器和输入布局，使用一次性 channel 将结果交回 GPUI。依赖 wallet-ledger，界面不自行计算或持久化流水。

通过根目录 `android/gradlew -p android assembleDebug` 构建。宿主机只运行 ledger 测试，不提供桌面入口。平台测试见 [Android 手动测试](../../android/manual-tests.md)。

钱包吉祥物基于已确认设计图，使用像素不变的无损 WebP，唯一资源位于 `android/app/src/main/res/drawable/ic_wallet.webp`。Android 图标与 GPUI 吉祥物共用该文件；Rust 启动时经 JNI 读取一次并持有解码缓存，不在原生库中重复嵌入。自定义钱包、统计图标来自 Lucide，许可证见 `assets/LICENSE-LUCIDE`；其他图标通过 gpui-kit assets 编译时嵌入。
