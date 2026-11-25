---
number: 1
title: "Linux/X11 启动崩溃：winit IME focus 抛出 GLXBadWindow 导致 panic；后台 dytes.bin 更新报 No such file or directory"
state: open
author: funxsayhi
labels: [bug]
created: 2026-11-25T01:38:47Z
updated: 2026-11-25T02:53:23Z
url: https://github.com/dytescn/Piksel/issues/1
---
# #1 Linux/X11 启动崩溃：winit IME focus 抛出 GLXBadWindow 导致 panic；后台 dytes.bin 更新报 No such file or directory

## 环境

- OS：Ubuntu，X11 桌面会话（`DISPLAY` 可用）
- Piksel：1.1.34（Linux 发布产物 `piksel.bin`）
- 同目录下存在 `cyctron.bin`、`dytes.bin`
- 启动方式：在安装目录直接执行 `./piksel.bin`

## 复现步骤

1. 将 piksel 1.1.34 的 Linux 产物解包到同一目录（`piksel.bin`、`cyctron.bin`、`dytes.bin`）。
2. 在 X11 环境下执行 `./piksel.bin`。
3. 窗口尚未正常显示，主线程在输入框/输入法获得焦点时崩溃。

## 实际结果

```
Program started, pid: 17416
Initializing ...
Trying to start: ./cyctron.bin
Exists: true
Running: dytes.bin
后台更新失败: No such file or directory (os error 2)
09:26:48 INFO  /home/runner/work/cyctron/cyctron/src/event_pusher.c:37: [NNG] Listening on ipc:///tmp/cyctron.sock
09:26:48 INFO  /home/runner/work/cyctron/cyctron/src/event_pusher.c:61: [EventPusher] Initialized
09:26:48 INFO  /home/runner/work/cyctron/cyctron/src/ws_worker.c:226: [WS] worker started (2 thread(s))
38da69e 3 mongoose.c:4801:mg_mgr_init   MG_IO_SIZE: 16384, TLS: none
38da69e 3 mongoose.c:4718:mg_listen     1 7 http://0.0.0.0:44944
09:26:48 INFO  /home/runner/work/cyctron/cyctron/src/main.c:63: HTTP + WS server started on http://0.0.0.0:44944

thread 'main' (17416) panicked at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/winit-0.30.13/src/platform_impl/linux/x11/event_processor.rs:1261:47:
Failed to focus input context: XError { description: "GLXBadWindow", error_code: 170, request_code: 152, minor_code: 32 }
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

## 期望结果

窗口正常打开并加载 `http://127.0.0.1:44944`。即使输入法（IME）上下文获取焦点失败，也应记录日志并继续运行，而不是 panic 退出。

## 原因分析

winit 0.30.13（0.30.12 代码相同）在 X11 的 `xinput2_focused` 中：

```rust
if let Some(ime) = wt.ime.as_ref() {
    ime.borrow_mut().focus(xev.event).expect("Failed to focus input context");
}
```

`Ime::focus` 实际调用 `XSetICFocus`，随后 `xconn.check_errors()`；它返回的是 X11 连接上**任意**未处理的异步错误。本例是 GLX 的 `GLXBadWindow`（`request_code: 152` 为 GLX 扩展）。`.expect()` 把这个与 IME 无关的错误升级为 panic，直接杀死主线程，进程退出。

- winit 0.30.12 与 0.30.13 均包含该 `expect`，仅升级 0.30.x 无法规避。
- 上游同类问题：<https://github.com/rust-windowing/winit/issues/3761>（fcitx/XIM 环境下同样报 `Failed to focus input context`）。
- 触发 `GLXBadWindow` 的具体 GLX 调用还需要进一步定位（可能来自 GTK/WebKitGTK 的 GL 初始化，或窗口尚未 map 完成时的 GLX 操作）；但 IME 的 focus 不应该把连接上无关的异步错误当成致命错误。

## 影响

panic 发生在事件循环内，`main` 中事件循环之后的清理逻辑不会执行：

- `pid_mgr.stop_cyctron()` 不会执行，`cyctron` 可能成为孤儿进程并继续占用 `127.0.0.1:44944`，导致下次启动端口冲突；
- `pid_mgr.clean_pid()` 不会执行，`run.pid` 残留。

## 建议

1. 向上游提交修复/打补丁：把 `focus` 失败降级为 `warn!`，不要 `.expect()`；在本仓库修复前可用 `[patch.crates-io]` 临时接管 winit。
2. 排查 `GLXBadWindow` 的来源与初始化顺序（GTK / WebKitGTK / `pixels`），例如改为窗口 map 完成后再创建 WebView / GL 上下文。
3. 在事件循环外层增加兜底清理（`catch_unwind` 或 panic hook 中终止 cyctron、清理 `run.pid`）。
4. 可临时验证：在有 Wayland 会话的机器上用 `WINIT_UNIX_BACKEND=wayland` 启动，确认是否绕过该路径。

---

## 附带问题：后台更新 `dytes.bin` 报 `No such file or directory (os error 2)`

同一份日志中：

```
Running: dytes.bin
后台更新失败: No such file or directory (os error 2)
```

对应 `src/core/pid.rs` 的 `spawn_background_update`：

```rust
let candidates = ["dytes", "dytes.exe", "dytes.bin", "dytes.app"];
let Some(exe) = candidates
    .iter()
    .copied()
    .find(|path| std::path::Path::new(path).is_file())
else { ... };

let mut cmd = std::process::Command::new(exe);
```

`is_file()` 检查的是**当前目录**，但 Unix 上 `Command::new("dytes.bin")` 中的名字不含 `/`，`execvp` 语义会按 **PATH** 查找，不会在当前目录执行。当前 PATH 中没有 `dytes.bin`，于是 spawn 返回 ENOENT。日志里打印了 `Running: dytes.bin`，说明存在性检查匹配到了当前目录的文件，执行时却按 PATH 查找，两者不一致。

对比 `start_cyctron` 的候选列表包含 `"./cyctron"`，所以 Linux 下 cyctron 能正常启动。

建议：

- Unix 下把候选写成 `./dytes.bin`，或者先 `canonicalize()` 为绝对路径再传给 `Command::new`；
- 或统一从 `std::env::current_exe()` 所在目录解析同级可执行文件；
- `start_cyctron` 与 `spawn_background_update` 建议共用同一套可执行文件查找逻辑，避免平台行为不一致。

## 评论

### @funxsayhi — 2026-11-25T02:05:28Z

补充本次复现后的残留状态（Ubuntu，Piksel 1.1.32，安装目录在 exFAT 移动硬盘 `/media/funx/dytes`）。

崩溃 30 分钟后：

```console
$ cat run.pid
17416
$ kill -0 17416
kill: (17416) - No such process

$ ps -p 17431 -o pid,ppid,lstart,cmd=
    PID    PPID                  STARTED CMD
  17431    1548 Wed Nov 25 09:26:48 2026 ./cyctron.bin start

$ ss -ltnp | grep 44944
LISTEN 0 128 0.0.0.0:44944 0.0.0.0:* users:(("cyctron.bin",pid=17431,fd=7))

$ ls -l /tmp/cyctron.sock
srwxrwxr-x ... /tmp/cyctron.sock
```

即：

- `17416`（Piksel）已不存在；
- `17431`（`./cyctron.bin start`）仍存活，父进程被 `systemd --user` 收养；
- `run.pid` 与 `/tmp/cyctron.sock` 均残留。

影响：下次启动会再拉起一个 cyctron，新实例绑定 44944 失败，WebView 实际连到旧实例；`start_cyctron()` 只检查 `spawn()` 成功，不检查服务是否可用。

建议在 panic/信号路径也执行清理（panic hook 终止 cyctron、删除 `run.pid`），并在 cyctron 侧增加父进程退出检测。相关后续问题已分别提交：

- #2 1.1.34 的升级断链（type 改为 core 后无法更新）
- #3 启动与退出健壮性（`EXIT_FLAG` 只写不读，SIGINT/SIGTERM 无法退出；cwd 依赖；WebView 失败 panic；忙等）
- #4 移动硬盘/exFAT 场景与发布配置
- funxdata/cyctron#8 父进程退出后 cyctron 不自退
- dytescn/dytes#34 安装器/更新器硬编码 `app/<name>/dytes.json`

### @dowell2020 — 2026-11-25T02:53:23Z

已在 **v1.1.36** 处理除 winit panic 之外的部分：

- **后台更新 ENOENT**：`dytes` / `cyctron` 统一从可执行文件目录解析为绝对路径，并在其所在目录运行，不再出现 `is_file()` 命中当前目录、`execvp` 却按 PATH 查找的失败。
- **panic 清理**：新增 panic hook，panic 时终止 cyctron 并删除 `run.pid`，避免孤儿进程占用 44944。
- **退出清理**：SIGINT / SIGTERM / 窗口关闭都会执行 `stop_cyctron()` + `clean_pid()`。
- **路径**：`run.pid` 不再依赖 cwd，固定在可执行文件目录。

winit 的 `Failed to focus input context`（GLXBadWindow）panic 仍存在：按维护要求不引入 vendor / 自造库，保持 crates.io 原生 winit 0.30.13（0.31 beta 仍有同样的 `expect`），需要上游修复。在此之前可用 Wayland 会话（`WINIT_UNIX_BACKEND=wayland`）规避该路径。
