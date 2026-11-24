# 代码规范

本文件记录 Piksel 项目的目录与代码组织规范。上游依据：dytes 仓库
`skills/code.md`（`../dytes/skills/code.md`），按 Piksel 的实际情况落地。

Piksel 是 `core` 类型的 Rust 应用（基于 cyctron + wry 的桌面壳），
主要目标平台为 Windows，同时保留 Linux / macOS 平台代码。

## 1. 目录结构

```text
src/
├── main.rs            # 入口：参数解析、启动
├── application/       # 应用生命周期与窗口（browser / vindu）
├── core/              # 核心运行时（app / pid）
├── platform/          # 平台实现（windows / macos / linux）
└── utils/             # 通用工具（ip 等）
```

新增代码按职责归入上述模块；跨模块的公共逻辑放在 `utils/`，不要反向依赖
`application/`。

## 2. `mod.rs` 只负责声明

`mod.rs` 只允许声明子模块（`pub mod`）、带 `cfg` 的模块声明与必要的重导出，
不允许写业务逻辑。

正确：

```rust
pub mod browser;
pub mod vindu;
```

错误：

```rust
pub mod browser;

pub fn run() { /* 逻辑代码 */ }
```

命令/功能实现放在独立文件中，模块入口只负责组装。

## 3. 测试代码不写在 `src/` 中

`src/` 下的源码文件不得包含 `#[cfg(test)] mod tests` 等测试代码；测试统一放在
仓库根目录的 `tests/` 下，按模块拆分为独立文件。

```text
tests/
├── cli.rs
├── manifest.rs
└── common/mod.rs      # 共享辅助代码
```

测试通过公共接口验证行为；需要测试内部实现时，优先重构为可测试的公共接口。

> 存量说明：当前 `src/` 中尚无测试代码，新增测试从第一条起就遵守本条。

测试环境、依赖与 `.dytes/` 约定见 [testenv.md](testenv.md)。

## 4. 平台相关代码统一放在 `src/platform/`

- 平台实现按 `#[cfg(target_os = "...")]` 分文件（`windows.rs` / `macos.rs` /
  `linux.rs`），不要在业务代码里散落平台分支。
- 各平台文件保持同一组导出签名（如 `create_window`），在 `platform/mod.rs`
  中按 `cfg` 重导出。
- 平台差异通过 `platform` 模块抽象，`application/` 与 `core/` 只依赖抽象层。

## 5. 版本号同步

对外版本号只维护两处，发布时保持一致：

- `Cargo.toml` 的 `[package] version`
- `dytes.json` 的 `version`

`Cargo.lock` 由构建生成（当前被 `.gitignore` 忽略），不手工维护。

## 6. CI 与发布文件

所有发布 / CI 配置统一放在 `.github/workflows/`，禁止放在仓库根目录。

- 新增同类文件直接建在 `.github/workflows/`；
- 仓库根目录只保留构建与包清单相关文件：`Cargo.toml`、`Cargo.lock`、
  `dytes.json`、`build.rs`、`README.md`、`LICENSE`；
- 文档与脚本引用时写完整路径，例如 `.github/workflows/release.yml`。

## 7. 依赖与清单

- 包依赖统一写在 `dytes.json` 的 `dependencies-core` / `dependencies-sys` /
  `dependencies-web` 中，Rust 依赖写在 `Cargo.toml`；两边不要混用。
- 依赖版本以线上发布为准，使用 `dytes update --check` / `dytes update` 更新，
  不要手改到不存在的版本。
- `dytes update` 会重写 `dytes.json`（包括键排序），执行后检查 `type` 与
  `version` 是否被回退，必要时恢复。
