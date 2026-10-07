---
title: 常见问题
description: YoYoVideo 的常见问题与排查方法。
---

# 常见问题

## 启动后没有画面，只有一片黑色

Windows 和 Linux X11 用 mpv 的 `wid`，macOS 用渲染 API。Wayland 单视频已接入实验性的
Slint/OpenGL 合成路径，但尚未实机验证；多宫格仍不支持。Wayland 上请确认使用支持 OpenGL 的
驱动和启用了 `mpv-runtime` 的构建，并查看状态栏及诊断日志中的合成初始化错误。
相关记录见 [`docs/development/runtime-dependencies.md`](https://github.com/ijry/YoYoVideo/blob/main/docs/development/runtime-dependencies.md)。

## 状态栏提示 "Playback runtime is disabled in this build"

你手上的二进制是用默认 feature 编译的，没有打开 `mpv-runtime`。用
[本地构建](/dev/build)一节的命令重新编译，或者直接下载发行包——发行包总是带播放内核的。

## 需要预装 mpv 或第三方解码器吗

不需要。libmpv 和它依赖的 FFmpeg 已经打包在 `bin/` 目录里。程序不会去读系统上安装的 mpv。

## 窗口位置跑到屏幕外了 / 窗口太小

播放器会把窗口尺寸限制在最小值，并在恢复位置时检查是否落在可见区域内。
如果仍然不对，删掉 `%APPDATA%\xyito\YoYoVideo\` 下的窗口状态文件再启动即可。

## 快捷键按了没反应

- 焦点可能不在播放器窗口上，先点一下画面区域。
- 某些绑定被改过了。到设置里的快捷键页面确认当前绑定。
- `Ctrl+` 开头的组合在部分终端或远程桌面里会被拦截，此时改用其他绑定。

## 拖进来的文件夹里有些文件没播放

扫描会跳过非媒体文件。批量模式有画面数量上限，超出的文件会在界面上提示被丢弃的数量，
而不是静默忽略。

## 会联网吗

本地播放无需联网，也不收集使用数据。自动/手动更新会访问 GitHub 获取版本信息和安装包，可在更新窗口关闭自动检查；不会上传媒体或播放历史。打开网络媒体地址则会连接你指定的来源。

## 可以商用吗

可以，但必须遵守 GPL-3.0-or-later：分发二进制时需要提供对应源码，包含你对该二进制的修改。
发行包里 `LICENSES/` 目录已经记录了运行时来源与许可证要点。
