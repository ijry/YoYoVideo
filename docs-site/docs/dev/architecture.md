---
title: 架构总览
description: YoYoVideo 的 crate 划分、数据流向与几个关键设计取舍。
---

# 架构总览

## 三个 crate

```
crates/yoyo-core          领域逻辑，不依赖播放内核
  ├── backend.rs          后端 trait 与事件定义
  ├── app_command.rs      命令枚举
  ├── shortcut.rs         快捷键映射
  └── progress.rs / sidebar.rs / track_popup.rs ...
        ▲
crates/yoyo-mpv           libmpv 适配层（feature: mpv-runtime）
        ▲
apps/yoyovideo-desktop   Slint 界面、平台集成、打包产物
```

## 为什么核心不碰 libmpv

`yoyo-core` 里没有任何一行 libmpv 代码，播放能力通过 `PlayerBackend` trait 抽象。
这带来两个直接好处：

1. **测试不需要内核。** 默认 feature 下 `cargo test` 用 dry-run 实现跑完全部用例，
   CI 的第一条流水线不必安装 libmpv。
2. **内核可以替换。** 换播放内核只需要新增一个 `yoyo-mpv` 之外的实现。

代价是启动时需要明确选择后端。没开 `mpv-runtime` 时，桌面端会报出明确的
"播放运行时未启用"错误，而不是假装能播。

## 播放内核的链接方式

`libmpv-sys` 在构建时发出 `cargo:rustc-link-lib=mpv`，也就是链接期需要一个名为 `mpv` 的库。
Windows 上具体是：

- `third_party/mpv/windows-x64/lib/mpv.lib` —— 链接期导入库
- `third_party/mpv/windows-x64/bin/mpv-2.dll` —— 运行期 DLL

这两者必须**同名对应**：可执行文件把导入库里的名字写进导入表，运行时就照这个名字去找 DLL。
上游 `shinchiro/mpv-winbuild-cmake` 的 mpv-dev 压缩包里给的是 MinGW 的 `libmpv.dll.a`
和名为 `libmpv-2.dll` 的 DLL，直接用会出现"MSVC 不认这个导入库"或"找不到 DLL"。
`scripts/fetch-runtime.ps1` 因此会把 DLL 改名为 `mpv-2.dll`，再用 `dumpbin` 导出表 +
`lib.exe /def:` 重新生成 MSVC 导入库，三者才对得上。

## 视频宿主

Windows 上可见视频走 mpv 的 `wid` 窗口绑定：Rust 创建一个原生子窗口，把窗口句柄交给 mpv，
由 mpv 往里渲染。界面本身只画控制条和背景，**画面区域是故意留空的**——否则 mpv 的窗口会被
Slint 的绘制盖住。弹出菜单时宿主窗口会被临时隐藏。

macOS 与 Wayland 尚未实现这条路径，程序会正常启动并在上层报告该限制。

## 界面

`apps/yoyovideo-desktop/ui/main-window.slint` 是单文件界面定义（约 77 KB）。
控件文案全部走 `root.ui_language_code == "zh" ? ... : ...`，中文优先，英文兜底。
无边框窗口的自绘标题栏、控制条自动隐藏逻辑在 `chrome_autohide.rs`。
