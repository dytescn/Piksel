---
number: 3
title: "启动与退出健壮性：SIGINT/SIGTERM 无法退出、依赖 cwd、窗口/WebView 失败会 panic、事件循环忙等"
state: open
author: funxsayhi
labels: [bug]
created: 2026-11-25T02:05:03Z
updated: 2026-11-25T02:53:44Z
url: https://github.com/dytescn/Piksel/issues/3
---
# #3 启动与退出健壮性：SIGINT/SIGTERM 无法退出、依赖 cwd、窗口/WebView 失败会 panic、事件循环忙等

## 环境

- Piksel 1.1.32 / main（1.1.34），Linux X11
- 以下代码位置均以 main 分支为准，1.1.32 同样存在

## 1. SIGINT / SIGTERM 无法退出程序

`src/core/pid.rs:116-139` 注册了信号处理函数：

```rust
static EXIT_FLAG: AtomicBool = AtomicBool::new(false);

pub fn setup_exit_handler() {
    unsafe {
        libc::signal(libc::SIGINT, exit_handler as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, exit_handler as *const () as libc::sighandler_t);
    }
    extern "C" fn exit_handler(_: i32) {
        EXIT_FLAG.store(true, Ordering::Release);
    }
}
```

但 `EXIT_FLAG` 在整个仓库里只被写入，没有任何地方读取。`libc::signal` 又覆盖了 SIGINT/SIGTERM 的默认终止行为，结果是：

- 终端 Ctrl+C 不会退出；
- `kill <pid>`（SIGTERM）也不会退出；
- 只能 `kill -9`，而 `kill -9` 会跳过 `stop_cyctron()` / `clean_pid()`。

建议：在事件循环中检查 `EXIT_FLAG` 并调用 `event_loop.exit()`，或者改用 `ctrlc`/信号线程 + `EventLoopProxy` 把退出请求投递到事件循环。

## 2. 运行依赖当前工作目录

多个路径是相对路径：

- `src/core/pid.rs:11`：`const PID_FILE: &str = "./run.pid";`
- `src/core/pid.rs:44-51`：`./cyctron`、`./cyctron.exe`、`./cyctron.bin`、`./cyctron.app`
- cyctron 自身也用 `./libary` 加载库

从桌面快捷方式、开机自启、或其他目录启动时：

```bash
cd ~ && /media/funx/dytes/piksel.bin
```

会写错 `run.pid` 位置（单实例失效），并且找不到 `./cyctron.bin`，窗口打开但页面无法加载。移动硬盘上通过文件管理器/快捷方式启动时尤其容易遇到。

建议：所有路径基于 `std::env::current_exe()?.parent()` 解析成绝对路径；`run.pid` 放在可执行文件目录或用户配置目录。

## 3. 窗口/WebView 创建失败会导致 panic，而不是降级

```rust
// src/core/app.rs:24-25
let window = self.window.as_ref().unwrap();
let webview = self.webview.as_ref().unwrap();
```

`resumed` 中 `vindu::init_vindu()` 与 `browser::create_webview()` 都用 `.ok()` 吞掉错误（`src/platform/linux.rs:9`、`src/application/browser.rs:14`）。若窗口或 WebView 创建失败，随后的 `WindowEvent::Resized` 会在 `render()` 里 `unwrap()` 直接 panic。

建议：`render()` 用 `let (Some(window), Some(webview)) = ... else { return }`；创建失败时记录明确日志并显示错误页/退出，而不是 panic。

## 4. 事件循环忙等

```rust
// src/core/app.rs:77-85
fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
    if let Some(window) = &self.window {
        window.request_redraw();
    }
    #[cfg(target_os = "linux")]
    while gtk::events_pending() {
        gtk::main_iteration_do(false);
    }
}
```

`about_to_wait` 每一轮都无条件请求重绘，而 `WindowEvent::RedrawRequested` 分支是空的。这会让事件循环持续自旋，占用 CPU、增加耗电（移动硬盘/笔记本场景明显）。建议只在尺寸变化或内容需要更新时请求重绘，或去掉无意义的 `request_redraw`。

## 5. 单实例检查不是原子的，且无法识别 PID 复用

```rust
// src/core/pid.rs:23-31
pub fn is_running(&self) -> bool {
    if !std::path::Path::new(PID_FILE).exists() { return false; }
    ...
    return is_process_alive(pid);   // kill(pid, 0) == 0
}
```

- 两个实例同时启动时都可能通过 `is_running()` 再各自写 `run.pid`；
- `kill(pid, 0)` 只能判断“该 PID 存在”，PID 被其他进程复用时会被误判为“Piksel 已在运行”。

建议：使用 `flock`/`fcntl` 对 `run.pid` 文件加独占锁（进程退出自动释放），或同时记录启动时间/可执行路径并校验。

## 关联

- issue #1：X11 下 winit IME focus 的 `GLXBadWindow` panic（当前已复现，且 panic 后不会执行清理）
- 本次扫描发现的孤儿 cyctron 进程，正是第 1 条“信号/退出路径不清理”的实证

## 评论

### @dowell2020 — 2026-11-25T02:53:43Z

已在 **v1.1.36** 修复全部五项：

1. **SIGINT / SIGTERM**：信号处理器只置原子标志，独立线程通过 `EventLoopProxy` 投递 `UserEvent::Exit`；Ctrl+C 和 `kill` 都会走正常退出清理，事件循环空闲时也能及时响应。
2. **cwd 依赖**：新增 `src/core/paths.rs`，`run.pid`、`cyctron`、`dytes` 均基于 `current_exe()` 所在目录解析（开发时回退到 cwd），启动子进程时设置 `current_dir`，满足 cyctron `./libary` 的加载方式。
3. **窗口 / WebView 失败**：`create_window` / `create_webview` 改为返回 `Result`，失败时记录具体错误并退出事件循环，`render()` 不再 `unwrap`。
4. **事件循环忙等**：移除无条件的 `request_redraw`；Linux 改为 16ms `WaitUntil` 定时泵送 GTK 事件，不再空转占满 CPU。
5. **单实例原子性**：改为文件锁（Unix `flock`、Windows `share_mode(0)` 独占打开），不支持文件锁的文件系统回退到 PID 存活检测；PID 复用误判基本消除。
