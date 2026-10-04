# 手动测试

先按 README 构建并安装 APK，在 ARM64 Android 模拟器或兼容手机启动。自动化业务测试执行 `cargo test -p wallet-ledger`。

平台操作步骤、输入值与预期结果见 [Android 手动测试](android/manual-tests.md)。测试钱包应在专门模拟器创建；不要清除用户设备上的真实账本。
