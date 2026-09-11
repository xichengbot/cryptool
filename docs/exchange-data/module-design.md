# exchange-data 模块设计

## 可达页面

实际菜单由 `fe/src/router/index.ts` 决定。

| 页面 | 路由 | 实际能力 |
|---|---|---|
| 首页 | `/dashboard` | 欢迎占位，无交易统计 |
| 现货历史成交 | `/binance/spot-history-trade` | ID / 时间查询、前后数量、结果表格、现货微秒显示逻辑 |
| 合约历史成交 | `/binance/future-history-trade` | ID / 时间查询、前后数量、结果表格、毫秒显示 |
| 时间转换 | `/tools/time-convert` | 文本 / Unix 输入转换、北京时间和 UTC 展示、内存历史列表 |

共同外壳包含侧栏、面包屑、页签、主题切换、全屏、菜单搜索、响应式布局。设置面板和通知默认隐藏，登录入口被注释。其他 table / permission / hook-demo / menu 演示文件未注册为当前业务路由。

## 成交页面

两个页面基本重复，默认 `type=ID`、`forward=50`、`behind=50`，各有 +50 按钮。表格包含原始时间、北京时间、tradeId、单价、数量、报价数量、maker 标志；现货额外展示 isBestMatch。表格使用 Element Plus 的边框、斑马纹和默认可调整列宽行为，没有服务端分页或导出功能。

输入转换目前在页面内部完成：包含 T 时用 Moment，含空格时用 Date，纯数字补齐 / 截断为 13 位再作为毫秒处理。`api/binance/index.ts::formatTimeMicro` 将现货结果直接按微秒解释。该文件未按时区显式固定北京时间。

## REST 接口

| 路径 | 方法 | 参数 |
|---|---|---|
| `/api/binance/spot/historicalTrades` | GET | symbol、type=ID/TIME、tradeId/time、forward、behind |
| `/api/binance/futures/historicalTrades` | GET | 同上 |

Controller 校验模式所需锚点非空，前后数量默认各 50，检查 `forward + behind <= 999`。没有负数、整数溢出或 symbol 空白校验。HTTP 响应直接返回 Service 给出的 SDK JSON 字符串 / null，没有独立业务 DTO。

## BinanceService

`searchSpotHistoryTrade` / `searchFutureHistoryTrade`：

1. TIME 模式请求 aggTrades，参数为 `[time,time+1000]`、limit=1。
2. 无聚合成交则返回 null；否则取第一条的 `f`。
3. 请求 historicalTrades，`fromId=anchor-forward`，`limit=forward+behind+1`。
4. 返回 SDK 原始字符串，前端负责字段展示。

没有 ID 下溢保护、分段分页、去重、缺口检查、退避或应用层缓存。

## 配置与异常

- `BinanceConfig` 注入现货 / 合约 SDK。合约 Bean 对 apiKey / secretKey 使用 `!!`，空配置可能在首次使用时失败。迁移到 Rust 时不能把 HMAC secretKey 当作 Ed25519 PEM 私钥。
- `ProxyConfig` 声明 HTTP / SOCKS / direct，包含用户名和密码配置；其 direct 分支仍构造带地址的 Proxy，行为需要修正后测试。
- `CorsConfig` 允许 `http://localhost:3333`，生产前端却也将 API 写死为 `http://localhost:8080/api`。
- `GlobalExceptionHandler` 将 BizException 按所携带状态返回，其他异常统一 500；日志只记录堆栈首帧和异常 message。
- 构建时 Maven 将前端 dist 复制到 Spring 资源目录，再生成 zip。构建后清空输出 YAML 凭据，不能消除源码配置中的非空值。

## 与 cryptool 的重叠

两者的“时间定位 → f → 原始成交窗口”流程一致。整合应复用 cryptool 的 reqwest、Decimal、领域映射和错误类型，增加 Web 边界、时间精度与窗口分页能力。旧 Kotlin 服务无需作为常驻依赖保留。
