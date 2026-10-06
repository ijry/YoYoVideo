---
title: 安装与下载
description: 下载 YoYoVideo v0.0.1 并在 Windows 上安装运行。
---

# 安装与下载

## 下载

前往 [Releases 页](https://github.com/ijry/YoYoVideo/releases/latest) 下载 v0.0.1。

| 文件 | 说明 |
| --- | --- |
| `YoYoVideo-windows-x64-setup.exe` | NSIS 安装包，安装后开始菜单有快捷方式 |
| `YoYoVideo-windows-x64.zip` | 免安装绿色包，解压即用 |

两种包内容完全一致，都包含 `bin/`（程序与播放内核）、`LICENSES/`（许可与运行时来源）和说明文档。

::: tip 不需要预装解码器
libmpv 及其依赖的 FFmpeg 已经打包在 `bin/` 目录里。系统上装没装 mpv、装没装第三方解码器，
都不影响 YoYoVideo 播放。
:::

## 校验

发行页每条资产都附有 SHA-256。下载后可以自行核对：

```powershell
Get-FileHash -Algorithm SHA256 .\YoYoVideo-windows-x64.zip
```

值应当与发行页"校验和 / SHA-256"一节中同名文件的值一致。

## 运行

**免安装包**：解压后双击 `bin\yoyovideo-desktop.exe`。

**安装包**：运行安装程序，安装完成后从开始菜单启动。

首次启动时窗口可能偏小或位置落在屏幕外，播放器会自行纠正；相关状态保存在
`%APPDATA%\xyito\YoYoVideo\` 下。

## 把文件交给播放器

- 双击 `bin\yoyovideo-desktop.exe` 后，把视频或整个文件夹拖进窗口。
- 在资源管理器里右键文件，选择"打开方式"并指定 `yoyovideo-desktop.exe`。
- 也可以在命令行传入文件；传入多个文件即进入批量播放：

```powershell
.\bin\yoyovideo-desktop.exe D:\videos\a.mp4 D:\videos\b.mkv D:\videos\c.mp4
```

## 其他平台

发行覆盖 Windows x64、macOS（Apple Silicon 与 Intel）和 Linux x64。视频怎么到达屏幕，各平台并不相同，
验证程度也不相同：

| 平台 | 视频路径 | 运行时 |
| --- | --- | --- |
| Windows x64 | mpv `--wid` | 固定上游构建的 DLL，随包捆绑 |
| Linux x64 | X11 下的 mpv `--wid` | 声明为 `.deb` 依赖 |
| macOS | mpv 渲染 API（OpenGL） | Homebrew dylib，随包捆绑并改写为 `@rpath` |

**两个需要说清楚的保留：**

- **不支持 Wayland。** 没有经过验证的宿主路径，程序会明确报告这个限制，而不是假装可用。
- **macOS 视频只通过了编译验证。** 渲染 API 这条路在两个架构上都能编译，但还没有在真机上跑过——
  CI runner 是无头的，根本不会创建窗口。在有人在真机上确认之前，请当作未验证。

如果你愿意补上这两块，可以从 [`runtime/manifest.toml`](https://github.com/ijry/YoYoVideo/blob/main/runtime/manifest.toml)
里对应条目的 `notes` 开始读——那里写清楚了缺什么。
