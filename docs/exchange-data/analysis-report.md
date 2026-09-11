# exchange-data 代码分析报告

## 概要

- 模式：标准；日期：2026-09-09；基线：`00dea82`。
- 范围：后端全部业务代码、三个实际业务页、路由 / 导航、请求封装、构建与配置、现有测试。
- 源码规模：Kotlin 394 行，Vue 4,244 行，TypeScript 2,487 行，含注释 / 空行；大部分前端为模板。
- 验证方式：静态分析和与 cryptool 已运行的本地模拟链路对照；未安装旧项目前端依赖，未启动 JVM 服务或运行其测试，不作运行通过声明。
- 参考仓库保持不变；分析文档存放在 cryptool 中。

## 架构评估

[架构概览](architecture.md)、[模块设计](module-design.md)、[数据流](data-flow.md) 已分别记录目录、依赖和执行链路。业务复杂度较小，适合用 Rust 组件复现三个业务页面，并用 Rust 服务承接两个 Kotlin 查询接口的功能。按用户选定方案，目标为 Dioxus Web + Axum 和 Dioxus Desktop；原 CLI 保留，其独有功能不增加图形界面入口。

现有系统是同步 Spring MVC + 单请求线程配置，前端无有效鉴权；无数据库和持久任务。Cache / Actuator 等依赖的存在不等于实现了业务缓存或完整可观测性。

## 发现问题

| 优先级 | 类别 | 问题 | 位置 | 整合时处理 |
|---|---|---|---|---|
| P1 | 敏感配置 | Git 跟踪的 YAML 中两个凭据字段有非空值，且非环境变量引用；真实性 / 有效性未验证 | `be/src/main/resources/binance.yml` | 不复制值；使用环境变量。若为有效凭据，应轮换并另行处理历史暴露 |
| P1 | 部署 | 生产 API 固定 localhost:8080，远端浏览器会访问自己的 localhost | `fe/.env.production` | 同源 `/api/v1` |
| P1 | 时间正确性 | 时间工具含 T 分支的 UTC 列未转 UTC，其他“北京时间”显示依赖浏览器时区 | `fe/src/views/tools/time-convert.vue:37` | 明确输入时区、固定输出时区、跨时区测试 |
| P2 | 时间单位 | 现货页面一律按微秒格式化，但应用层没显式请求微秒响应 | `fe/src/api/binance/index.ts:29`、`BinanceConfig.kt:23` | 由 Rust adapter 显式选择单位，返回单位元数据 |
| P2 | 输入 | 非数字 ID 提示后继续请求；formRules 未在查询前调用 validate | 两个成交页的 `handleSearch` | 输入组件和后端双重验证 |
| P2 | 输入 | 普通 el-input 用于数量，编辑后可能变字符串，+50 有拼接风险 | 两个成交页的 `forwardPlus` / `behindPlus` | 数字输入、整数 / 非负 / 总量限制 |
| P2 | 窗口 | 前后数可为负，求和无 checked 运算，fromId 可为负 | `BinanceCtrl.kt:45`、`BinanceService.kt:54` | 受验证窗口类型、饱和边界、明确不足记录的含义 |
| P2 | 接口适配 | 合约接受至 1000 条却只发一个请求，未适配当前单页 500 的限制 | `BinanceCtrl.kt:75`、`BinanceService.kt:93` | 1000 条逻辑窗口分多页请求 |
| P2 | 运行效率 | Tomcat 最大线程数为 1，同步 SDK 网络调用串行占用线程 | `be/src/main/resources/application.yml` | Rust 异步网络 + 有界并发 / 请求预算 |
| P2 | 配置 | 合约配置 apiKey / secretKey 使用非空断言，缺配置首访失败 | `BinanceConfig.kt:49` | 只对真正需要凭据的 endpoint 检查，错误明确 |
| P2 | 测试 | Kotlin 测试全部注释，前端测试为基础工具 / 模板，没有业务回归 | `be/src/test/`、`fe/tests/` | 以功能矩阵建立 core / API / UI 验收 |
| P3 | 维护性 | 现货和合约页面及查询服务逻辑重复 | 两个成交 Vue 页面、`BinanceService.kt` | 共用表单 / 表格组件和 Rust 查询服务 |

合约单页限制依据 [Binance USDⓈ-M 市场数据文档](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/market-data)，于 2026-09-09 核对。

## 安全与依赖边界

没有发现 SQL 或 shell 命令拼接的业务入口。未启用身份认证，CORS 配置仅允许一个开发 origin，但 CORS 不能替代身份认证。原始异常 message 直接返回客户端，后续 Web 服务应使用结构化、脱敏错误。

源码凭据与构建时清空产物是两件事：清空 target 下的 YAML 不会清除 Git 中的值。此分析没有修改、撤销或验证任何凭据。

依赖版本从 pom.xml / package.json 核对，未进行漏洞数据库扫描，不凭版本年龄判定存在具体 CVE。前端继承 MIT 模板许可；迁移代码时保留 LICENSE 和版权声明。

## 改进顺序

1. 按可达路由与 API 建立复现清单，区分实际业务和模板演示。
2. 在 cryptool 中补齐时间精度、1000 条逻辑窗口、可靠输入校验与代理配置。
3. 增加 HTTP / Native 两种查询适配，复用相同应用服务；用 Dioxus 重建三个业务页面及导航外壳。
4. 保持旧 CLI 默认参数和输出数据字段；Web / Desktop 只覆盖 exchange-data 的实际功能。
5. 用离线固定响应、跨时区测试和双 renderer 验收，分别发布 Web 服务版与独立桌面应用。

## 附录

核心后端：`BinanceCtrl.kt`、`BinanceService.kt`、`BinanceConfig.kt`、`ProxyConfig.kt`。

核心前端：`router/index.ts`、两个 `views/binance/*history-trade.vue`、`views/tools/time-convert.vue`、`api/binance/index.ts`、`utils/service.ts`。

依赖摘要见 [架构概览](architecture.md)。完整功能矩阵、目标技术栈和迁移验收见 [整合设计](../superpowers/specs/2026-09-09-exchange-data-web-integration-design.md)。
