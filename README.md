# Piksel

> Co-Design Software —— Funxdata 桌面应用体系的桌面外壳。

Piksel 是一个用 Rust 编写的轻量桌面容器：它只负责**原生窗口、WebView、进程托管与更新调度**，不实现业务界面。启动后，Piksel 会拉起本地服务 `cyctron`，再把 WebView 指向 `http://127.0.0.1:44944`，由 Web 层（`init`、`pages`、`stylesheet` 等）渲染全部界面。

## 在整体架构中的位置

```text
用户双击 piksel.exe
        │
        ├─ 单实例检查（run.pid）
        ├─ update.exe --auto --startup      检查并安装更新
        ├─ cyctron start                    启动本地服务 127.0.0.1:44944
        └─ 原生窗口 + WebView ──────────────▶ Web UI
        │
退出时：终止 cyctron 子进程，清理 run.pid
```

| 组件 | 角色 |
| --- | --- |
| **Piksel** | 桌面外壳：窗口、WebView、单实例、更新与 `cyctron` 生命周期 |
| [cyctron](https://github.com/funxdata/cyctron) | 本地服务与运行时，监听 `127.0.0.1:44944` |
| [update](https://github.com/dytescn/piksel_install) | 更新器，由 Piksel 以 `--auto --startup` 调用 |
| [init](https://github.com/dytescn/init) 等 Web 包 | 前端初始化与界面，由 WebView 加载 |

## 主要能力

- **原生窗口 + WebView**：基于 `winit` + `wry`，Windows / macOS / Linux 共用一套窗口与尺寸同步逻辑。
- **单实例运行**：通过 `run.pid` 记录进程号，并用系统调用（Windows `OpenProcess`、Unix `kill(pid, 0)`）判断其是否存活，避免重复启动。
- **启动自更新**：启动时先用 `update.new.exe` 替换旧的 `update.exe`，再同步执行 `update.exe --auto --startup`。
- **服务托管**：以子进程方式运行 `cyctron start`；Windows 下不弹出控制台窗口，主进程退出时终止并回收该子进程。
- **注入桌面标识**：向页面注入 `window.Piksel_DESKTOP = true`，Web 层可据此启用桌面端行为。
- **跨平台适配**：Linux 下初始化 GTK 并在事件循环中泵送 GTK 事件，避免 WebView 闪烁；退出时注册 `SIGINT` / `SIGTERM` / Windows 控制台事件处理。
- **开发调试**：WebView 默认开启 devtools。

## 目录结构

```text
Piksel/
├── src/
│   ├── main.rs                 # 入口：单实例、事件循环、cyctron 启停
│   ├── core/
│   │   ├── app.rs              # 窗口/WebView 生命周期与 resize 同步
│   │   └── pid.rs              # PidManager：单实例、更新、cyctron 托管
│   ├── application/
│   │   ├── browser.rs          # 创建 WebView（URL、注入脚本、devtools）
│   │   └── vindu.rs            # 窗口创建入口（转交 platform）
│   ├── platform/
│   │   ├── windows.rs          # Windows 窗口（800×600）
│   │   ├── macos.rs            # macOS 窗口（1280×800）
│   │   └── linux.rs            # Linux 窗口（800×600）
│   └── utils/
│       └── ip.rs               # 本机 IPv4 查询（预留工具）
├── icons/                      # Windows 图标资源
├── index.html                  # 应用入口页，加载 init/init.js
├── dytes.json                  # 包描述、依赖与发布导出配置
├── build.rs                    # Windows 图标资源编译
├── Cargo.toml
└── example/                    # 本地运行示例（已被 .gitignore 排除）
```

> 早期用于下载页的 `load_pages/`（Deno + TypeScript 实现的静态页面）已移除，本仓库现在只保留桌面外壳本身。

## 构建

环境要求：

- Rust（edition 2021，建议使用较新的 stable 工具链）
- **Windows**：WebView2 运行时
- **Linux**：GTK3 与 WebKit2GTK 开发包，以及 `pkg-config`

```bash
cargo build --release
```

产物位于 `target/release/piksel.exe`（Windows）或 `target/release/piksel`（macOS / Linux）。

## 运行

Piksel 需要在**工作目录**中找到同级的 `cyctron` 与 `update`，并加载 `index.html` 及 Web 资源。典型目录：

```text
piksel/
├── piksel.exe / piksel
├── cyctron.exe / cyctron
├── update.exe / update
├── index.html
└── run.pid                  # 运行时自动生成
```

直接执行 `piksel.exe` 即可；首次运行前请确认 `cyctron` 可用，否则窗口虽会打开，但页面无法加载。推荐的完整安装 / 更新 / 发布流程请参考 [piksel_install](https://github.com/dytescn/piksel_install) 或 `dytes_sys_operations` 中的相关文档。

`example/` 目录保存了一套可运行的本地示例（含 `cyctron`、`update` 与 Web 资源），但不纳入版本控制。

## 配置：dytes.json

`dytes.json` 描述包的身份、版本、依赖与发布产物：

```json
{
  "name": "piksel",
  "version": "1.1.13",
  "type": "app",
  "dependencies-core": { "cyctron": "1.0.13", "update": "1.0.0" },
  "dependencies-web": { "init": "1.0.20", "pages": "1.0.0", "...": "..." },
  "exports": {
    "./index.html": "index.html",
    "./target/release/piksel.exe": "piksel.exe"
  },
  "auto_update": true
}
```

- `dependencies-core` / `dependencies-sys` / `dependencies-web`：安装时由 `piksel_install` 解析并拉取。
- `exports`：发布时把构建产物映射到包内路径。
- `auto_update`：允许 `update` 在启动时自动检查更新。

## 发布

发布由 [`.github/workflows/release.yml`](.github/workflows/release.yml) 统一完成，**只在打 tag 或手动触发时运行**，日常提交不会触发构建。

```bash
# 先同时升级 Cargo.toml 与 dytes.json 的 version，再打同名 tag
git tag v1.1.13
git push origin v1.1.13

# 或手动重跑：GitHub → Actions → Release Piksel → Run workflow
```

流程：

1. `build` 矩阵并行构建 Windows / macOS / Linux，并上传二进制到对象存储：

   ```text
   core/piksel/<version>/win|linux|mac
   ```

2. `publish-manifest` 在三个平台全部构建成功后，上传 `dytes.json` 到 `app/piksel.json`（构建失败不会覆盖线上清单）。

构建前会校验：

- `Cargo.toml` 与 `dytes.json` 的 `version` 必须一致；
- tag 发布时，tag 名必须是 `v` + 版本号（如 `v1.1.13`）。

> 发版前务必**同时**更新 `Cargo.toml` 与 `dytes.json` 中的 `version`；仅替换二进制而不升版本，`update.exe` 会认为“已是最新”而跳过更新。

## 许可证

[MIT](./LICENSE) © 2025 Funxdata
