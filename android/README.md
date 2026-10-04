# Android 工程

Android Studio 打开本目录。ARM64、最低 Android 8 / API 26，要求 Vulkan。Gradle JDK 17–24；当前验证环境为 Apple Silicon + JDK 24 + API 34 模拟器。

`app/build.gradle.kts` 的 buildRustDebug / buildRustRelease 任务调用根目录构建脚本并复制动态库。WalletActivity 承担网络与系统栏外观；WalletDocuments 负责系统文件选择器及后台导入导出；GpuiInputActivity 桥接中文 IME、小数键盘及显示／收起；GpuiPlatformView 提供无嵌入媒体情况下的生命周期入口。界面和业务均在 Rust crate。

运行 `./gradlew assembleDebug` 构建调试版，`./gradlew assembleRelease` 构建发布版、`./gradlew installDebug` 安装。启动 Activity：`adb shell am start -n app.jizhang.wallet/.WalletActivity`。详见[手动测试](manual-tests.md)。
