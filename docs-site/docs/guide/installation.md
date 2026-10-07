---
title: 安装与下载
description: YoYoVideo 的平台包与自动更新。
---

# 安装与下载

新的 0.0.1 发布流程使用 Velopack；请以 [Releases](https://github.com/ijry/YoYoVideo/releases/latest) 实际已发布资源为准。

| 平台 | 文件 | 更新方式 |
| --- | --- | --- |
| Windows x64 | `YoYoVideo-stable-windows-x64-Setup.exe` | 安装后应用内更新 |
| macOS ARM / Intel | 各架构独立 `Portable.zip`，内含 .app | 应用内更新 |
| Linux x64 | `YoYoVideo.AppImage` | 可写位置支持应用内更新 |
| Linux x64 | `YoYoVideo-linux-x64.deb` | 包管理器或手动更新 |

发行页提供 SHA-256，例如 Windows 下载后可核对：

```powershell
Get-FileHash -Algorithm SHA256 .\YoYoVideo-stable-windows-x64-Setup.exe
```

Windows 安装后从开始菜单启动；macOS 解压后将 .app 放在可写应用目录；Linux AppImage 需要执行权限。
本地播放所需 libmpv 随 Windows/macOS/AppImage 包分发，.deb 使用发行版依赖。
启动后点击中间的“打开文件”按钮或拖入媒体文件/文件夹。

## 自动更新

通过“检查更新”打开更新窗口，可关闭自动检查、下载、稍后安装或确认退出安装。
安装前验证签名和包哈希；未确认的下载不会在下次启动时自动安装。设置和历史保留，更新后不会自动播放。
旧 NSIS、普通 ZIP 和开发构建不能原地迁移到新格式，请手动安装新的发行包。

macOS 使用 **ad-hoc 签名，未做 Apple 公证**，首次启动可能被 Gatekeeper 拦截。
请只从可信发布页下载，按系统正常审批流程处理；不建议关闭安全检查。
Windows 当前也没有受信任 Authenticode 证书，可能有 SmartScreen 提示；更新签名不等于系统代码签名。

AppImage 的构建基线为 Ubuntu 22.04，仍需要系统图形驱动、桌面和音频环境。
打包通过不等于各平台实际安装升级都已验收，详见[开发验证状态](https://github.com/ijry/YoYoVideo/blob/main/docs/development/updater.md)。

## 其他平台

发行覆盖 Windows x64、macOS（Apple Silicon 与 Intel）和 Linux x64。视频怎么到达屏幕，各平台并不相同，
验证程度也不相同：

| 平台 | 视频路径 | 运行时 |
| --- | --- | --- |
| Windows x64 | mpv `--wid` | 固定上游构建的 DLL，随包捆绑 |
| Linux x64 | X11 下的 mpv `--wid` | AppImage 捆绑非宿主依赖；.deb 声明系统依赖 |
| macOS | mpv 渲染 API（OpenGL） | Homebrew dylib，随包捆绑并改写为 `@rpath` |

**两个需要说清楚的保留：**

- **Wayland 仍属实验性。** 单视频已接入 Slint/OpenGL 合成，尚未经过 Wayland 实机验证；多宫格仍不支持。
- **macOS 视频只通过了编译验证。** 渲染 API 这条路在两个架构上都能编译，但还没有在真机上跑过——
  CI runner 是无头的，根本不会创建窗口。在有人在真机上确认之前，请当作未验证。

如果你愿意补上这两块，可以从 [`runtime/manifest.toml`](https://github.com/ijry/YoYoVideo/blob/main/runtime/manifest.toml)
里对应条目的 `notes` 开始读——那里写清楚了缺什么。
