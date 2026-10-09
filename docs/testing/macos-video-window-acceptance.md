# macOS 原生视频窗口回归

此回归使用真实 macOS WindowServer、libmpv 和应用的原生窗口，不以 `cargo check` 或模拟窗口代替。GitHub Actions 在 Apple Silicon 和 Intel runner 分别执行，发布工作流也将其作为发布前置条件。

## 本机执行

需要 Rust、PowerShell 7、mpv 和 ffmpeg。`Platform` 根据本机架构选择 `macos-aarch64` 或 `macos-x86_64`：

```powershell
brew install mpv ffmpeg
pwsh -NoProfile -File scripts/fetch-runtime.ps1 -Platform macos-aarch64 -Force
cargo build --locked -p yoyovideo-desktop --features privacy-qa
pwsh -NoProfile -File scripts/test-macos-video-window.ps1 -Platform macos-aarch64
```

`privacy-qa` 仅用于隔离验收，不能发布给用户。脚本先检查 `--build-info`，拒绝驱动普通发行版；它在本仓库 `.cache` 内新建带标记的配置、时钟与合成红/蓝影片，不读取用户的设置或媒体。

## 验证范围

- 空启动只有播放器可见，视频宿主保持隐藏。
- 空窗口移动、缩放、最小化和恢复后不出现黑色矩形。
- 使用真实影片和播放进度，检查宿主属于主窗口，并与实际主窗口内容区对齐。
- 播放时移动、缩放主窗口，宿主随之定位；重复同步不能抢走键盘焦点。
- 菜单遮挡、最小化和恢复后，宿主重新挂接并保持正确位置。
- 单视频宿主与渲染上下文正常关闭。
- 坐标单元测试另覆盖 Retina 比例、翻转坐标系和非零视图原点；这不等于实际多显示器/Retina 设备验收。

## 证据

每次运行的目录为 `.cache/macos-window-<id>`，包含：

- `report.json`：通过的检查、失败原因、最后状态。
- `events.jsonl`：实际 AppKit 窗口的坐标、父窗、可见性与焦点，以及播放状态。
- `player.stdout.log`、`player.stderr.log`。
- 进程卡住时尽可能生成 `player.sample.log`，用于区分 UI/mpv 互相等待与单纯窗口定位错误。

CI 附件名为 `native-video-window-macos-aarch64` / `native-video-window-macos-x86_64`。其中只上传隔离测试的日志和状态，不上传用户媒体。

## 维护注意

1. AppKit 子窗口使用屏幕坐标，不是 Win32/X11 的父窗客户区坐标。必须通过主窗口内容视图转换，并采用主窗口的 backing scale。
2. `NSWindow.orderOut()` 不仅隐藏窗口，还会解除父子关系。因此要保存正确的父窗，并在允许显示时重新挂接。
3. winit 在 macOS 上的 `request_redraw()` 虽然线程安全，却会同步切到主线程。mpv 更新回调必须只异步投递重绘，否则可能与主线程上的 mpv 操作互锁。
5. 此回归不等于完整 macOS 隐私验收：PIN、所有受限周期切换、系统级截图或 Apple 公证不在此处的验证范围内。
