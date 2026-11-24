# 测试环境规范

本文件约定 Piksel 的测试分层、测试产物管理与排障流程。上游依据：dytes 仓库
`skills/testenv.md` 与 `docs/commands/test.md`，按 Piksel 的 `core` 应用实际情况落地。

## 1. 测试分层

| 层次 | 命令 | 说明 |
| --- | --- | --- |
| 代码测试 | `cargo test --locked` | 单元 / 集成测试，测试代码放 `tests/` |
| 静态检查 | `cargo clippy --all-targets -- -D warnings` | 合入前必须无警告 |
| 格式检查 | `cargo fmt --check` | 需要时用 `cargo fmt` 修正 |
| 清单校验 | `dytes check` | 校验 `dytes.json`、类型、平台导出 |
| 包安装演练 | `dytes test` | 在线下载依赖到 `.dytes/env/`，演练安装布局 |
| 依赖版本检查 | `dytes update --check` | 只检查，不修改清单 |
| 运行时冒烟 | `cargo run` 或 `target\release\piksel.exe` | 需要图形环境，CI 中跳过 |

发布前至少通过：`cargo test` + `dytes check` + `dytes test` + `dytes update --check`。

## 2. 测试目录 `.dytes/`

所有本地测试产物统一放在包目录的 `.dytes/` 下，由 `dytes test` 自动创建：

```text
.dytes/
├── env/                    # 测试依赖环境（每次 test 重建）
│   ├── dytes.json          # 合并后的测试依赖清单
│   ├── dytes.lock          # 依赖安装记录
│   ├── libary/             # sys / core 依赖
│   ├── web/<name>/         # web 依赖
│   ├── plugins/<name>/     # design_plugins 依赖
│   ├── apps/<name>/        # design_soft 依赖
│   └── models/<name>/      # model 依赖
└── run-<pid>/
    └── dytes-test-<时间戳>/ # 当前包的安装演练目录
```

- `.dytes/` 不存在时由 `dytes test` 自动创建，并自动写入根 `.gitignore`；
- `.dytes/env/` 每次执行前清空重建，避免旧依赖残留；
- 演练目录默认结束即删，`--keep` 保留用于排查；
- `.dytes/` 是生成目录，可随时整体删除。

Piksel 已在 `.gitignore` 中加入 `.dytes/`，测试产物不得提交。

## 3. 测试依赖 `test-dependency`

仅测试需要、且不属于正式依赖的包，写在 `dytes.json` 的 `test-dependency` 中：

```json
"test-dependency": {
  "sys": { "info_lib": "0.1.0" },
  "web": { "setting": "0.2.0" }
}
```

- 合并规则：并入 `dependencies-*`，同名条目以 `test-dependency` 为准；
- 当前包自身不下载，始终使用本地构建产物；依赖按声明 semver 下载线上最新版；
- 任一依赖下载失败即中止，不回退本地复制。

## 4. 环境隔离

- `DYTES_CONFIG` 指向 `.dytes/.../config.toml`，禁止读写真实配置目录
  （`%APPDATA%\dytes`、`~/.config/dytes`）；
- 清除外部环境变量：`DYTES_S3_*`、`DYTES_PLATFORM`、`PIKSEL_PLATFORM`，
  避免本机环境影响结果；
- 平台相关用例固定 `DYTES_PLATFORM=windows`，或按用例显式指定；
- 默认不访问网络；需要真实 S3 的网络用例必须显式标记并默认跳过。

## 5. 测试代码

- 测试统一放在 `tests/`，不写入 `src/`（见 [code.md](code.md) 第 3 节）；
- 共享辅助代码放在 `tests/common/mod.rs`；
- 调用真实二进制的集成测试使用 `CARGO_BIN_EXE_piksel`，验证退出码、
  stdout/stderr 与文件结果；
- Windows 专属测试用 `#[cfg(windows)]` 标注；Linux 依赖 gtk，不在本机默认执行。

## 6. 没有 S3 配置或网络不通时

先区分「凭据问题」和「网络问题」，不要直接重装或反复重试：

1. 检查凭据与端点：

   ```bash
   dytes config list
   ```

   本地 `cargo build` 默认不内置 S3 凭据，只有构建时提供 `UPYUN_API_KEY` /
   `UPYUN_API_SECRET` 才会注入。需要时：

   ```bash
   dytes config set s3.access_key
   dytes config set s3.secret_key
   ```

   或使用 `DYTES_S3_ACCESS_KEY` / `DYTES_S3_SECRET_KEY`。

2. 检查网络与 DNS：

   ```bash
   curl -s -o /dev/null -w "%{http_code}\n" https://s3.api.upyun.com/
   ```

   `Connection refused` 但 ping 正常时，优先怀疑 `hosts` 把域名钉到了失效 IP：

   ```text
   C:\Windows\System32\drivers\etc\hosts
   ```

   本项目曾因 `117.148.177.198 s3.api.upyun.com` 导致全部请求失败，
   改为可用的 `115.231.101.162` 后恢复。修改前先备份。

3. 排除 CDN 缓存，用带时间戳的 URL 确认真实版本：

   ```bash
   curl -fsSL "https://s3.api.upyun.com/packages/core/piksel/dytes.json?t=$(date +%s)"
   ```

4. 只测本地逻辑时，使用 `.dytes/` 隔离目录与 `DYTES_CONFIG`，不依赖线上包。

## 7. 发布后验证

1. 同步 `Cargo.toml` 与 `dytes.json` 的版本号，提交并打 tag；
2. 等待 `.github/workflows/release.yml` 完成，确认三平台产物与清单已更新；
3. 用当前 dytes 发布并验证：

   ```bash
   dytes check
   dytes publish --dry-run
   dytes publish
   ```

4. 复制一份包到临时目录或使用 `.dytes/`，执行安装演练：

   ```bash
   dytes test
   dytes update --check
   ```

5. Piksel 本身是 `core` 类型包，不通过 `dytes self-update` 升级；dytes CLI 的
   升级用 `dytes self-update --check` / `dytes self-update`。

## 8. 清理与不提交项

以下为 `dytes update` / `dytes test` 生成的本地产物，不应提交：

```text
.dytes/
libary/
models/
web/
database/
designdb/
dytes.lock
piksel.exe
dytes.exe
cyctron*.exe
```

需要重置本地环境时，直接删除上述目录与文件后重新执行 `dytes update` 或
`dytes test`。
