# 小钱包

用 Rust + GPUI Kit 制作的 Android 竖屏余额记账应用。人民币／美元钱包、改名与删除、余额更新、收入记录、日月年统计、资产图表、导入导出和浅深色外观。数据保存在设备本地。

从 [Releases](https://github.com/yIllusionSky/jizhang/releases) 下载 ARM64 APK。支持 Android 8 及以上、具备 Vulkan 的 ARM64 手机。当前 GPUI Android 支持仍属实验阶段，已通过模拟器验证，尚未覆盖实体手机和 TalkBack。

## 开发

用 Android Studio 打开 `android/`。需要 macOS 或 Linux、Rust **1.98.1**、JDK **21**（兼容 17–24）、Android SDK 35、NDK 27.0.12077973。设置 `JAVA_HOME` 与 `ANDROID_HOME` 指向本机安装目录。

```sh
rustup target add aarch64-linux-android
./android/gradlew -p android assembleDebug
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
cargo test --locked -p wallet-ledger
```

Gradle 自动编译 Rust。Android Studio 内置 JDK 若为 25 或更高，请在 Gradle JDK 设置中选择 JDK 21。

## 发布

更新 Cargo workspace 版本与 `CHANGELOG.md`，提交后推送对应 `vX.Y.Z` tag。GitHub Actions 自动构建、签名并发布 Android APK 和 SHA-256 校验文件。仅构建手机端 ARM64；签名配置与发布步骤见[运行维护](docs/operations.md)。

## 目录

- [crates/ledger](crates/ledger/README.md)：金额、流水、汇率折算、统计和本地存储。
- [crates/app](crates/app/README.md)：GPUI Kit 界面与 Android 入口。
- [android](android/README.md)：Android Studio 工程、系统键盘与网络桥接。

参见[业务规则](docs/features.md)、[架构](ARCHITECTURE.md)和[手动测试](manual-tests.md)。
