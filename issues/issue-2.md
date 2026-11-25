---
number: 2
title: "1.1.34 将包类型改为 core 后，已安装的 1.1.32 无法通过 dytes update 升级"
state: open
author: funxsayhi
labels: [bug]
created: 2026-11-25T02:05:01Z
updated: 2026-11-25T02:53:42Z
url: https://github.com/dytescn/Piksel/issues/2
---
# #2 1.1.34 将包类型改为 core 后，已安装的 1.1.32 无法通过 dytes update 升级

## 环境

- Piksel 1.1.32（Linux，安装在移动硬盘 `/media/funx/dytes`，exFAT）
- 安装方式：`dytes` 安装器，安装日志 `install.log`
- 工具链：`dytes` 0.2.25
- 目标版本：GitHub tag `v1.1.34` 已存在（2026-11-24）

## 现象

已安装的 1.1.32 无法升级到 1.1.34，`dytes update` 认为 Piksel 已是最新。

安装日志中获取的远端清单仍是 1.1.32：

```text
==== Piksel 安装日志 1795569745 ====
安装目录: /media/funx/dytes
获取应用配置: app/piksel/dytes.json
应用配置: {
  "name": "piksel",
  "version": "1.1.32",
  ...
}
应用配置已写入: /media/funx/dytes/dytes.json
开始执行内置更新引擎（等价 dytes update --auto）
```

日志在“开始执行内置更新引擎”之后再无输出，安装目录里的 `dytes.json` 仍为 1.1.32。

在安装目录检查更新：

```console
$ ./dytes.bin update --check --json
{"check":true,"count":2,"types":[],"updates":[
  {"local_version":null,"name":"piksel","needs_update":false,"remote_version":null,"type":"app"},
  {"local_version":null,"name":"cyctron","needs_update":true,"remote_version":"1.0.18","type":"core"},
  {"local_version":null,"name":"dytes","needs_update":true,"remote_version":"0.2.25","type":"core"},
  ...
]}
```

`piksel` 被判定为 `needs_update: false`，因此永远不会升级。

## 根因

1.1.34 的提交 `9941efa3`（“release: 1.1.34（dytes 依赖升级到 0.2.24，包类型改为 core）”）把 `dytes.json` 的 `type` 从 `app` 改成 `core`，并同步修改了 `.github/workflows/release.yml`：

```diff
- destination_dir: app/${{ steps.version.outputs.project }}/${{ steps.version.outputs.version }}/${{ matrix.platform }}
+ destination_dir: ${{ steps.version.outputs.type }}/${{ steps.version.outputs.project }}/${{ steps.version.outputs.version }}/${{ matrix.platform }}
...
- destination_dir: app/${{ steps.package.outputs.project }}
+ destination_dir: ${{ steps.package.outputs.type }}/${{ steps.package.outputs.project }}
```

于是 1.1.34 的二进制与清单被上传到 `core/piksel/...`，而 1.1.34 之前版本的安装/更新逻辑只查询 `app/piksel/...`：

- `dytes` 安装器：`src/install/installer.rs:116,123`，固定 `app/{app}/dytes.json`
- `dytes` 应用更新：`src/update/update_app.rs:31-33` 的 `fetch_remote_app_info`，固定 `app/{name}/dytes.json`
- `dytes` 更新检查：`src/update/dytes.rs:664-680` 的 `app_need_update`，用本地 `dytes.json` 的 `version` 与 `app/{name}/dytes.json` 比较

线上 `app/piksel/dytes.json` 仍然是 1.1.32，所以：

- 新安装：仍然拿到 1.1.32；
- 已安装 1.1.32：本地 1.1.32 == 远端 1.1.32，`needs_update = false`。

1.1.34 的清单实际在 `core/piksel/dytes.json`，上述任何路径都不会读到它。

## 影响

- 1.1.32 及更早的安装无法通过 `dytes update` 升级到 1.1.34（升级断链），只能手动替换二进制；
- 1.1.34 中 `type: core` 与 1.1.34 之前的安装器/更新器不兼容，属于破坏性发布变更；
- 1.1.34 的更新路径变更没有迁移方案，也没有兼容旧路径的双写。

## 建议

- 如果 Piksel 仍然是桌面应用（不是依赖库），把 `dytes.json` 的 `type` 改回 `app`，并恢复发布路径为 `app/piksel/...`；
- 如果确实要改成 `core`，发布时同时写入 `app/piksel/dytes.json` 与 `core/piksel/dytes.json`（至少一个发布周期），并让 `dytes` 的安装/更新逻辑按清单中的 `type` 或同时探测两种路径；
- 在 release workflow 中增加发布后校验：`app/piksel/dytes.json` 的 `version` 必须等于本次 tag，否则失败；
- 发布 1.1.35 时确保旧客户端能读到新版本号，验证 1.1.32 → 1.1.35 的实际升级路径。

关联：本仓库 issue #1（1.1.34 仍未修复启动崩溃）。

相关工具侧问题已同步到 dytescn/dytes（安装器/更新器硬编码 `app/` 路径）。

## 评论

### @dowell2020 — 2026-11-25T02:53:42Z

已在 **v1.1.36** 修复升级断链：

- `dytes.json` 的 `type` 改回 `app`，发布路径恢复为 `app/piksel/...`；1.1.34 之前的安装/更新逻辑只读取该路径。
- 版本升到 **1.1.36**（`Cargo.toml` 与 `dytes.json` 同步），1.1.32 等已安装版本可以通过 `app/piksel/dytes.json` 看到新版本并升级。
- `release.yml` 增加 `type` 必须为 `app` 的发布前校验，避免再次误改。
- 移除 `publish-manifest` 的 `if: always()`：构建失败时不再覆盖线上清单。

`core/piksel` 下遗留的 1.1.34 清单仍建议在 dytes 侧做路径迁移或清理。
