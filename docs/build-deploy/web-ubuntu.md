# Web 构建与 Ubuntu 部署

Web 页面由 Dioxus 编译为 WASM 并嵌入 Axum 二进制，不需要 Node 或单独复制静态资源。

本机构建：

```sh
./scripts/web/build-local.sh
./dist/web-local/cryptool --config ./config.toml serve
```

使用已安装的 Zig 和 cargo-zigbuild 交叉构建 Ubuntu x86_64/glibc 2.17 发布包：

```sh
cargo install cargo-zigbuild --locked
./scripts/web/build-ubuntu.sh
```

产物：`dist/cryptool-web-linux-x86_64.tar.gz` 和对应 `.sha256`。脚本检查 ELF 架构、动态
解释器、最高 GLIBC 版本、压缩包和校验和。目标服务器可先验证：

```sh
sha256sum -c cryptool-web-linux-x86_64.tar.gz.sha256
tar -xzf cryptool-web-linux-x86_64.tar.gz
cd linux-x86_64
cp config.example.toml config.toml
```

配置本地 7890 HTTP 代理：

```toml
[binance.proxy]
mode = "http"
url = "http://127.0.0.1:7890"
```

同目录后台运行并允许局域网访问：

```sh
CRYPTOOL_LISTEN=0.0.0.0:8080 ./cryptool-web-server.sh start
./cryptool-web-server.sh status
./cryptool-web-server.sh restart
./cryptool-web-server.sh stop
```

自定义 Host 或端口：

```sh
CRYPTOOL_LISTEN=0.0.0.0:9090 \
CRYPTOOL_ALLOW_HOSTS="192.168.1.129,exchange.local" \
./cryptool-web-server.sh start
```

端口检查：`sudo ss -ltnp | grep ':8080'`。脚本优先读取同目录的 `cryptool` 和
`config.toml`，PID 与日志也写在同目录。Linux 上若出现 `Exec format error`，说明复制了
macOS/Windows 或错误 CPU 架构的二进制，应使用本脚本生成的 ELF x86-64 产物。

如果目标机保留了项目仓库，可用仓库部署脚本把实际解压目录部署到 `/data/cryptool`；服务器已有
`config.toml` 会被保留：

```sh
sudo ./scripts/web/deploy-ubuntu.sh \
  --source /path/to/linux-x86_64 \
  --target /data/cryptool \
  --user cryptool \
  --start
```

直接运行二进制时可用 `serve --listen 0.0.0.0:8080 --allow-host <服务器IP>`。生产环境建议
由 systemd 执行 `cryptool-web-server.sh run`，并由 Nginx 提供 TLS 与访问控制。
