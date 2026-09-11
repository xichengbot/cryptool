# Exchange Data 依赖审计

日期：2026-09-09。工具：cargo-audit 0.22.2；使用当日更新的 RustSec 数据库，对整个 Cargo.lock 执行审计。Cargo.lock 包含未启用的可选依赖和未交付平台依赖，因此同时检查实际 feature / target 依赖图。

数据库提交：`d502590ca247f3e53b56bf6c2ae40b61926800e5`，共 1,242 条 advisory。h2 补丁更新后，查询 / 代理 / API 的 26 项相关测试及后端严格 Clippy 通过，Web 与 macOS 发布包重新构建成功。

## 已修复

`h2 0.4.15` 受 [RUSTSEC-2026-0258](https://rustsec.org/advisories/RUSTSEC-2026-0258.html) 影响：特制 HTTP/2 空 DATA 帧可能造成无界内存使用或溢出 panic。它实际进入原生 HTTP 客户端依赖图，已最小升级至 `0.4.16`；只更新版本及 checksum，不升级其他无关依赖。

## 剩余原始告警

原始 `cargo audit` 仍以退出码 1 报告 [RUSTSEC-2026-0235](https://rustsec.org/advisories/RUSTSEC-2026-0235.html)：锁文件中的 `rkyv 0.7.46` 处理不可信归档时可能越界读，修复线为 0.8.17 及以上。

该版本来自 `rust_decimal 1.42.1` 未启用的可选 `rkyv` feature。CLI default、server / embed-web、macOS Desktop、Web WASM，以及 workspace all-features / all-target 的反向依赖图均无 rkyv 路径；应用没有归档反序列化入口。它没有进入本次支持配置的可执行依赖图。没有为了清零审计结果而引入无关的大版本升级，也没有在仓库中设置全局忽略。

用于区分当前产物风险的有限范围命令：

```sh
cargo audit --ignore RUSTSEC-2026-0235
```

该命令退出码为 0，但不等价于原始审计全绿。如果未来启用 rkyv feature、增加归档读取，必须重新评估并升级；不能沿用当前不可达判断。

## 警告与维护状态

| 依赖 / RustSec | 类型 | 本次产物可达性 |
|---|---|---|
| fxhash / [2025-0057](https://rustsec.org/advisories/RUSTSEC-2025-0057.html) | 不再维护 | Android Desktop 路径，本次不交付 |
| paste / [2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html) | 不再维护 | Desktop 图像依赖中的构建宏，实际可达；上游维护风险保留记录 |
| proc-macro-error / [2024-0370](https://rustsec.org/advisories/RUSTSEC-2024-0370.html) | 不再维护 | GTK 的 Linux / BSD Desktop 路径，本次不交付 |
| glib / [2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) | unsound | GTK 的 Linux / BSD Desktop 路径，本次不交付 |
| rand 0.7.3 / [2026-0097](https://rustsec.org/advisories/RUSTSEC-2026-0097.html) | unsound | Android Desktop 的 selectors 构建依赖，本次不交付 |

另有 yanked `chacha20 0.10.1` 锁文件条目，来自未启用的 QUIC 可选依赖路径；支持构建及 workspace 全 feature / target 反向图均无实际路径。审计没有发现需要为此启用或改造 QUIC 的理由。

## 适用边界

审计结论只覆盖本次锁文件与当前 RustSec 数据库，不能代表未来没有新漏洞。Windows 安装和实机运行未验证；这与锁文件审计的执行结果分别记录。原生 UI 使用上游系统 WebView，其操作系统更新不由 Cargo.lock 管理。
