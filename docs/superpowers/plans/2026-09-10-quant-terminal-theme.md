# 高频量化终端主题实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 为 Exchange Data 实现机构量化浅色主题、黑客终端暗色主题和“鲸”字 Web favicon。

**架构：** 保留现有 Dioxus 组件与主题切换状态，仅调整 CSS 设计令牌和主题选择器。favicon 作为 Dioxus 静态资产写入文档头，并在 Web bundle 后嵌入 Axum 发布二进制。

**技术栈：** Dioxus 0.7.10 document/asset、SVG、CSS 自定义属性、Dioxus CLI、cargo-zigbuild。

---

## 文件结构

- 创建 `apps/exchange-ui/assets/favicon.svg`：单字“鲸”浏览器图标。
- 修改 `apps/exchange-ui/src/app.rs`：向文档头注册 favicon。
- 修改 `apps/exchange-ui/assets/main.css`：浅色与黑客暗色主题、应用外壳和状态反馈。
- 修改 `apps/exchange-ui/assets/components.css`：表单、表格、分页和标记的主题细节。
- 更新 `dist/linux-x86_64/`：重新构建并封装 Ubuntu Web 发布包。

### 任务 1：接入“鲸”字 favicon

- [x] 创建 64×64 SVG：深色圆角底和居中的“鲸”字，使用系统中文无衬线字体回退。
- [x] 在 `app.rs` 声明 `FAVICON: Asset`，用 `document::Link` 设置 `rel="icon"` 和 `type="image/svg+xml"`。
- [x] 运行 `xmllint --noout apps/exchange-ui/assets/favicon.svg`，预期退出码 0。
- [x] 运行 Web WASM `cargo check`，预期编译成功。

### 任务 2：实现量化终端色系

- [x] 在 `.app` 定义冷白、海军蓝、青蓝、涨跌和警告设计令牌。
- [x] 在 `.app.dark` 定义近黑绿、荧光青绿、冷青、涨跌和警告令牌。
- [x] 调整侧栏、页签、按钮、面板和状态栏，使它们只消费设计令牌。
- [x] 为暗色工作区增加低对比度静态网格；不使用闪烁、循环动画和荧光辉光。
- [x] 调整表格斑马纹、锚点行、状态徽标和滚动容器，保证暗色主题可读性。

### 任务 3：构建与视觉验收

- [x] 运行 `cargo fmt --check`、Web WASM `cargo check` 和相关库测试。
- [x] 运行 `./scripts/build-web.sh`，确认输出包含哈希化 favicon SVG。
- [x] 启动本地 Axum Web 服务，在桌面视口检查浅色、暗色主题与 favicon，并复核移动断点。
- [x] 运行 `cargo zigbuild -p cryptool --release --locked --features embed-web --target x86_64-unknown-linux-gnu.2.17`。
- [x] 更新 Ubuntu 部署目录、压缩包和 SHA-256 文件，并验证 ELF 格式及压缩包完整性。
