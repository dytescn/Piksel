---
number: 4
title: "移动硬盘（exFAT）安装场景：无持久日志、非真正便携、生产开启 devtools、清单与文档不一致"
state: open
author: funxsayhi
labels: [bug]
created: 2026-11-25T02:05:05Z
updated: 2026-11-25T02:05:05Z
url: https://github.com/dytescn/Piksel/issues/4
---
# #4 移动硬盘（exFAT）安装场景：无持久日志、非真正便携、生产开启 devtools、清单与文档不一致

## 环境

- Ubuntu，Piksel 1.1.32 安装在移动硬盘 `/media/funx/dytes`
- 文件系统：exFAT，挂载参数 `rw,nosuid,nodev,relatime,uid=1000,gid=1000,fmask=0022,dmask=0022`
- 系统库：WebKitGTK 4.1、GTK3

## 1. 无持久运行日志，移动场景难以排查

Piksel 与 cyctron 的日志只写 stdout/stderr，安装目录里只有安装器生成的 `install.log`（且该文件在“开始执行内置更新引擎”处中断）。崩溃后终端关闭，`run.pid`、孤儿进程、端口冲突等问题没有任何可追溯记录。

建议：增加可配置的日志目录（默认 `~/.local/state/piksel/` 或安装目录 `logs/`），至少记录启动、cyctron 启停、窗口/WebView 创建失败、更新调用结果、退出原因。

## 2. 不是真正便携：依赖宿主机运行库

`piksel.bin` 是动态链接的可执行文件：

```console
$ readelf --version-info piksel.bin | grep GLIBC_ | sort -V | tail -1
GLIBC_2.39
$ readelf -d piksel.bin | grep NEEDED
libwebkit2gtk-4.1.so.0
libgtk-3.so.0
...
```

换到 Ubuntu 22.04（glibc 2.35）或未安装 WebKitGTK 4.1 的机器会直接启动失败。移动硬盘的常见预期是“插上就能用”，当前实现不满足。

建议：在 README/安装文档中明确最低系统要求；或提供 AppImage/自带运行库的打包方式。

## 3. 没有移动介质拔出/挂载点消失的处理

安装目录可能被卸载或重新挂载到不同路径。Piksel 没有检测：

- cyctron 的数据库与 `libary/*.so` 都在移动硬盘上；
- 拔出后写入失败没有明确提示，可能只表现为窗口空白或请求 500。

建议：启动时记录并校验安装目录所在设备；运行中对关键写入失败给出明确错误；文档提示必须安全弹出。

## 4. 生产构建开启 devtools

```rust
// src/application/browser.rs:9
.with_devtools(true);
```

发布版同样可以通过快捷键打开开发者工具，能访问本地页面与调试接口。建议只在 debug 构建或显式环境变量下开启。

## 5. 未使用的依赖

`Cargo.toml` 中以下依赖在 `src/` 里没有引用：

- `clap`
- `image`
- `http`
- `http-range`
- `libloading`
- `pixels`
- `serde` / `serde_json`

其中 `pixels` 会引入 GL 相关依赖，增大构建体积和攻击面。建议清理，或说明保留原因。

## 6. 清单与文档不一致

- 顶层 `README.md` 的配置示例仍是 `"version": "1.1.31"`、`"type": "app"`，与 1.1.34 的 `"type": "core"` 不一致；
- `dytes.json` 的 `exports` 只声明 Windows 产物，Linux/macOS 只写在 `platform-exports` 中，容易让发布/安装工具误判；
- 安装目录没有 `.desktop` 文件或启动器，双击 `piksel.bin` 时的 cwd 不确定（与本仓库的“启动与退出健壮性”issue 叠加）。

建议：统一 README 与清单中的版本/类型示例；明确 `exports` 与 `platform-exports` 的优先级；提供 Linux `.desktop` 文件并固定 `Path=` 为安装目录。
