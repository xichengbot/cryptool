# Exchange Data 验收记录

日期：2026-09-09。实现分支：`feat/exchange-data`。所有交易查询使用本机 Rust 模拟交易所和虚构 Key，未读取真实凭据或修改参考仓库。

## 自动化检查

以下集成命令通过，共 57 个测试（CLI / 原生服务 / API 38、共享类型 9、Desktop UI 逻辑 10）：

```sh
cargo test -p cryptool -p exchange-types -p exchange-ui --no-default-features --features cryptool/server,exchange-ui/desktop --locked
cargo clippy -p cryptool -p exchange-types -p exchange-ui --all-targets --no-default-features --features cryptool/server,exchange-ui/desktop --locked -- -D warnings
cargo fmt --all -- --check
```

Web WASM check 与严格 Clippy 通过。测试覆盖 CLI 参数与 JSON 回归、精度和时间校验、分页预算、取消释放、429 冷却、实际 HTTP / SOCKS5h 代理握手、Host / Origin / GET body / HEAD / 静态路由边界。

工具链固定为 Rust 1.98.0，本机与 CI 版本一致。[依赖审计](exchange-data-dependency-audit.md) 已执行：更新实际使用的 h2 安全补丁；原始审计仍报告未启用的可选 rkyv 条目，有限范围审计通过，未将原始结果描述为全绿。[本地性能记录](exchange-data-performance.md) 包含包体、DOM 更新、并发保护、进程启动和主进程 RSS，并明确测量边界。

后端审查发现并修复了 HEAD 隐式执行查询、无 framing headers 的非空 GET body 校验缺口；已补回归并通过复审。UI 审查补齐了原生全屏状态同步、多进程偏好文件临时路径和导航可访问性。

## Web 运行验收

- [x] Dioxus 发布包与内嵌资源的 Axum 二进制构建成功；关闭发布调试符号后 wasm-opt 正常执行。
- [x] Axum 内嵌资源程序从 `/tmp` 启动，不依赖仓库工作目录；深链接返回页面，未知资源与 API 返回 404，HEAD 查询返回 405。
- [x] 现货 ID 查询默认前后各 50 条，返回 101 条；表格首屏 100 条，下一页 1 条，锚点跳转与突出显示正常。
- [x] `9007199254740993` 精确显示；价格、数量的尾零保留；现货微秒、合约毫秒及市场专属列正确。
- [x] USD-M 前 499 / 后 500 共 1,000 条，实际两次上游请求；100 条与全部显示切换正常。
- [x] TIME 查询通过聚合 `f` 再核对原始时间；微秒锚点 `1700000000123458` 定位 ID 1002，窗口为 1000–1004。
- [x] 超过 1,000 条的表单不发送请求；`+50` 按整数更新；空结果、无效交易对、429 冷却和保留旧结果的提示正常。
- [x] 浏览器模拟 1 秒网络延迟后重新提交，旧请求产生 `net::ERR_ABORTED`，最终只显示新 ID 1001 的结果。
- [x] 时间转换验证微秒、负时间、无效输入、最新记录置顶和跨页面历史保留；主题偏好独立存储。
- [x] 390 像素窄屏下页面无横向溢出；表格独立滚动。浅色与深色布局已查看。
- [x] 菜单搜索过滤与导航状态正常；浏览器全屏可进入和退出；实际拖动后 ID 列宽从 176 增至 266 像素。

## 桌面和平台限制

- [x] macOS x86_64 发布 `.app` 构建成功，从 `/tmp` 启动后实际创建 ExchangeData 窗口。
- [ ] macOS 窗口内容和原生交互尚未完整验收：系统拒绝窗口截图，当前工具无法验证桌面视觉内容；共享页面已在 Web 验证，原生服务有独立测试。
- [ ] Windows 构建、安装和实机运行尚未验证。已配置 Windows CI，当前 macOS 宿主没有执行远端 CI；不能将配置存在视作 Windows 通过。

桌面仅支持 macOS / Windows，不构建 Linux Desktop。macOS 签名 / 公证、Windows 签名及公开分发不在本次本地验证范围。真实 Binance 网络和真实 API Key 权限未做实测。

## 收尾

- [x] 全分支最终审查及后续修复复审通过，无未解决的 Critical / Important / Minor 代码问题；平台验收限制仍按上文保留。
- [x] h2 安全补丁后的 Web / macOS 最终发布包构建成功；内嵌四份资源逐字节核对，WASM MIME 与独立目录查询验证通过。

已知上游提示：`block 0.1.6` 有 Rust future-incompatibility notice；当前构建和严格 Clippy 通过。
