# 运行维护

构建脚本 `scripts/build-native.sh` 默认使用 `ANDROID_HOME=$HOME/Library/Android/sdk` 和 `ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.0.12077973`。可显式设置这两个变量。Linux 默认 SDK 路径为 `$HOME/Android/Sdk`，也兼容 `ANDROID_SDK_ROOT`；Android Studio 的 Gradle 进程需要 PATH 中有 cargo 和 rustup 配置的工具链。

Gradle JDK 使用 17–24。如提示 Java 25/26 不兼容，请在 Android Studio 的 Build Tools → Gradle → Gradle JDK 中选择兼容 JDK。SDK 路径可在 `android/local.properties` 的 `sdk.dir` 设置；该文件不提交。

本地记录位于应用私有 `files/wallets-v1.json`。写入使用临时文件、fsync 和原子重命名。卸载会删除记录，Android 云备份已关闭。调试版可备份：

```sh
adb shell am force-stop app.jizhang.wallet
adb exec-out run-as app.jizhang.wallet cat files/wallets-v1.json > wallets-backup.json
```

文件包含钱包名称及余额，按个人财务数据妥善保存。不要直接编辑运行中应用的数据文件。

日志：

```sh
adb logcat -s jizhang AndroidRuntime
```

汇率源为 `https://api.frankfurter.dev/v1/latest?base=USD&symbols=CNY`，无 API 密钥。失败时先检查设备网络和系统时间。若主机通过 HTTP 代理联网，模拟器可能需要以 `emulator -avd <名称> -http-proxy http://127.0.0.1:<端口>` 启动；不要为排障关闭 TLS 验证。

## APK 发布

`assembleDebug` 编译调试 Rust 库；`assembleRelease` 编译优化后的 release 库。GitHub Release 只由 `vX.Y.Z` tag 触发，先创建草稿，构建和上传全部成功后公开。普通 push 不构建；面向 main 的 PR 执行格式、业务测试和 Android 调试构建。

仓库 Actions secrets：

- `ANDROID_KEYSTORE_BASE64`：发布 JKS 的 Base64 内容。
- `ANDROID_KEYSTORE_PASSWORD`：JKS 密码。
- `ANDROID_KEY_ALIAS`：签名别名。
- `ANDROID_KEY_PASSWORD`：私钥密码。

签名文件和密码必须在仓库外妥善备份；更换签名会导致已有安装无法覆盖升级。工作流临时解码签名文件，完成后清理。调试版与发布版签名不同，首次从调试版切换到发布版需要先备份数据并卸载调试版。

本地签名发布还需设置 `ANDROID_KEYSTORE_FILE` 指向 JKS 文件，以及上述密码与别名环境变量。没有签名配置时，本地 `assembleRelease` 生成未签名 APK；工作流强制校验签名配置并用 apksigner 验证最终产物。

发布步骤：修改根 Cargo.toml 的 workspace 版本，用 cargo check 更新 Cargo.lock，更新 CHANGELOG 对应版本区块并提交，然后执行：

```sh
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z
```

工作流校验 tag、Cargo 版本和 changelog 一致，Android versionCode 使用 `major × 1000000 + minor × 1000 + patch`；minor 和 patch 必须小于 1000。APK 文件名含版本和架构，并附 `SHA256SUMS.txt`。失败时 Release 保留为草稿；修复环境问题后可重跑同一 tag，不覆盖已公开的 Release。

Apple Silicon 模拟器应使用 Lavapipe 软件 Vulkan，当前模拟器的 host / SwiftShader 模式可能发生设备丢失或着色器失败。启动辅助脚本：

```sh
scripts/preview-android.sh Pixel_3a_API_34_extension_level_7_arm64-v8a
```

无窗口测试可追加 `-no-window -crash-report-mode never`，仍可用 adb 安装和截图。
