# wallet-ledger

独立 Rust 业务库，公开 Money、Currency、Ledger、Period、Summary、Rate 和 Store。负责精确金额解析、钱包余额流水、人民币折算、日期统计和原子 JSON 存储，不依赖 Android、GPUI 或网络。

在仓库根目录运行 `cargo test -p wallet-ledger`。业务修改应覆盖同日多次操作、余额校正、跨日期沿用和历史汇率快照。
