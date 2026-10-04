# 架构

```mermaid
flowchart TD
    Android[Android NativeActivity / IME / HTTPS] --> Mobile[gpui-mobile]
    Mobile --> UI[crates/app · GPUI Kit]
    UI --> Ledger[crates/ledger]
    Ledger --> Store[设备私有目录 wallets-v1.json]
    UI --> HTTP[Android HTTPS 后台调用]
    HTTP --> FX[Frankfurter / ECB]
```

`ledger` 不依赖 GPUI 或 Android。金额使用整数分，解析不经浮点数；汇率使用百万分之一精度。每笔操作记录余额、性质与当时汇率，历史收入不会被后续汇率改写。统计按设备本地日期计算，未更新日期沿用余额。

`app` 的 WalletApp 持有界面状态和 Ledger。保存先复制账本，验证并原子写入成功后再替换界面状态。文件损坏时保留原文件并禁止写入。无云端数据服务、用户账号或后台常驻服务。

汇率在后台线程请求，Android 提供证书验证和连接超时。启动时和前台跨日时检查同步日期；请求失败保持缓存，不生成虚假汇率。首次美元钱包要求至少一个有效汇率。应用关闭期间不会定时唤醒；重新打开时补同步。

Android 工程负责包装 Rust 动态库。gpui-kit 固定 0.7.0；gpui-mobile 固定提交 `9075e3aa3eea812127f2c60ed66f0cd5798ff245`。原生控件、图表均由 GPUI 渲染，PNG 仅用于钱包吉祥物和启动图标。Java IME 适配器基于该提交的 GpuiInputActivity 并增加数字键处理。

测试以 ledger 的真实业务序列和持久化重载为主，平台输入、滚动与图表通过 Android 模拟器测试。
