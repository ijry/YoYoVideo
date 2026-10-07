<p align="center">
  <img src="docs-site/docs/public/logo.svg" width="72" alt="YoYoVideo">
</p>

<h1 align="center">YoYoVideo 悠悠乐播</h1>

<p align="center">
  Rust + Slint + libmpv 打造的全格式本地视频播放器。<br>
  多画面批量播放 · 字幕与音轨切换 · 画面滤镜 · A-B 循环 · 完全离线
</p>

<p align="center">
  <a href="https://github.com/ijry/YoYoVideo/releases/latest"><img alt="下载" src="https://img.shields.io/github/v/release/ijry/YoYoVideo?label=%E4%B8%8B%E8%BD%BD&style=flat-square"></a>
  <a href="https://ijry.github.io/YoYoVideo/"><img alt="文档" src="https://img.shields.io/badge/%E6%96%87%E6%A1%A3-ijry.github.io%2FYoYoVideo-38bdf8?style=flat-square"></a>
  <a href="LICENSE"><img alt="许可证" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue?style=flat-square"></a>
</p>

---

## 界面

启动后的默认界面。视频区为空，控制条在鼠标静止数秒后自动隐藏。

![YoYoVideo 默认界面](docs/assets/player-default.png)

## 这是什么

一个**本地**视频播放器：界面用 [Slint](https://slint.dev/) 绘制，播放交给
[libmpv](https://mpv.io/)。它主要解决一件市面产品普遍没做好的事——**同时看多个视频**：

- **多画面批量播放**。拖入多个文件或整个文件夹，每个画面是独立播放会话，各自播放、暂停、
  跳转、调音量，单格尺寸可以拖拽调整。
- **完整播放控制**。精确跳转、倍速、缩放、旋转、画面平移、A-B 循环、章节跳转、时间轴标记点。
- **字幕与音轨**。外部字幕加载，字幕延迟 / 缩放 / 垂直位置可调，音轨与声道模式即时切换。
- **画面工具**。常用滤镜预设，以及亮度、对比度、饱和度调节。
- **轻量无边框**。自绘标题栏与控制条，滚轮调音量，悬停呼出菜单。
- **完全离线**。没有网络请求，没有遥测。libmpv 运行时随包分发，**无需预装任何解码器**。

完整功能与快捷键见[文档站](https://ijry.github.io/YoYoVideo/)。

## 下载

v0.0.1 发布 **Windows x64** 便携包与 NSIS 安装包，解压即用。

| 文件 | 说明 |
| --- | --- |
| `YoYoVideo-windows-x64-setup.exe` | 安装包 |
| `YoYoVideo-windows-x64.zip` | 免安装绿色包 |

前往 [Releases](https://github.com/ijry/YoYoVideo/releases/latest) 下载，发行页附有每个文件的 SHA-256。

> **为什么只有 Windows？** macOS 目前没有经过审核的通用架构 libmpv 构建，且原生视频嵌入尚未实现；
> Linux 需要打包 libmpv 的完整依赖闭包，Wayland 单视频已有实验性的 OpenGL 合成路径，但尚未经过 Wayland 实机验证，多宫格仍不支持。
> 详见 [`runtime/manifest.toml`](runtime/manifest.toml) 里对应条目的 `notes`。

## 从源码构建

需要 Rust stable（edition 2024）。Windows 上还需要 Visual Studio Build Tools 的 C++ 工作负载
——`libmpv-sys` 链接时要用 MSVC 工具链。

```powershell
# 测试：默认 feature 下不需要 libmpv
cargo test --workspace
cargo fmt --check

# 拉取播放内核（公开上游构建，按 sha256 固定）
pwsh -NoProfile -File scripts/fetch-runtime.ps1 -Platform windows-x64

# 带真实播放运行
pwsh -NoProfile -File scripts/dev-run.ps1

# 打包
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime
```

> 用 `dev-run.ps1` 而不是直接 `cargo run`：`mpv-runtime` 不能做成默认 feature，否则没有 libmpv 的
> 机器上 `cargo test` 会直接失败。`dev-run.ps1` 用独立的 `--target-dir`，避免普通
> `cargo build` 把同一个二进制**不带** `mpv-runtime` 重新编译并悄悄覆盖掉。

## 工作区结构

| 路径 | 作用 |
| --- | --- |
| `crates/yoyo-core` | 播放会话与命令的领域逻辑，**不依赖任何播放内核** |
| `crates/yoyo-mpv` | libmpv 适配层，把内核事件翻译成领域事件 |
| `apps/yoyovideo-desktop` | Slint 桌面应用、平台集成、打包产物 |
| `scripts/` | 运行时拉取、打包、校验、冒烟测试 |
| `runtime/manifest.toml` | 播放内核的来源、版本与校验和 |
| `docs-site/` | 文档站（VitePress） |

`yoyo-core` 里没有一行 libmpv 代码，所以整个播放领域逻辑可以在没有内核的机器上被完整测试——
这是默认 `cargo test` 能跑起来的前提。

## 贡献

提交前请确保：

```powershell
cargo fmt --check
cargo test --workspace
```

CI 会在 Windows / macOS / Linux 上检查 `mpv-runtime` feature 能否编译。
发版流程见[文档站的发布页](https://ijry.github.io/YoYoVideo/dev/release)。

## 许可证

YoYoVideo 以 **GPL-3.0-or-later** 发布，完整文本见 [LICENSE](LICENSE)。

发行包内捆绑的播放内核来自 [`shinchiro/mpv-winbuild-cmake`](https://github.com/shinchiro/mpv-winbuild-cmake)，
其中的 mpv 与 FFmpeg 均为 **GPL-2.0-or-later**。分发二进制时必须同时提供对应源码，
详见 [LICENSES.md](LICENSES.md) 与发行包内的 `LICENSES/` 目录。

[English](README-EN.md) · [文档站](https://ijry.github.io/YoYoVideo/)
