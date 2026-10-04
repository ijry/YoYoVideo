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

v0.0.1 **只发布 Windows x64**。这不是疏忽：

- macOS 目前没有经过审核的通用架构 libmpv 构建（上游只提供 Windows 构建，Homebrew 只有分架构的 bottle），
  而且原生视频嵌入在 macOS 上尚未实现，程序会正常启动但不显示视频画面。
- Linux 需要打包 libmpv 的完整依赖闭包，且 Wayland 下的原生嵌入同样尚未实现。

如果你愿意协助这两个平台，可以从 [`runtime/manifest.toml`](https://github.com/ijry/YoYoVideo/blob/main/runtime/manifest.toml)
里对应条目的 `notes` 开始读——那里写清楚了缺什么。
