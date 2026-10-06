---
title: 本地构建
description: 在本地编译、运行与打包 YoYoVideo。
---

# 本地构建

## 依赖

- Rust stable（edition 2024）
- Windows 上还需要 Visual Studio Build Tools 的 C++ 工作负载——`libmpv-sys` 链接时需要 MSVC 工具链
- PowerShell 7（脚本以 `pwsh` 调用）
- Windows 打包播放内核时需要 7-Zip

## 跑测试

默认 feature 下播放走 dry-run seam，**不需要 libmpv**：

```powershell
cargo test --workspace
cargo fmt --check
```

这是整个测试策略的前提：`crates/yoyo-core` 里没有一行 libmpv 代码，所以领域逻辑可以在
任何机器上被完整测试。

## 带真实播放运行

需要先把播放内核放进 `third_party/mpv/windows-x64/`：

```powershell
pwsh -NoProfile -File scripts/fetch-runtime.ps1 -Platform windows-x64
```

然后：

```powershell
pwsh -NoProfile -File scripts/dev-run.ps1
pwsh -NoProfile -File scripts/dev-run.ps1 D:\videos\a.mp4 D:\videos\b.mp4
```

::: warning 用 dev-run.ps1，不要直接 cargo run
`mpv-runtime` 不能做成默认 feature——那会让 `cargo test` 在没有 libmpv 的机器上直接失败。
`dev-run.ps1` 用独立的 `--target-dir`，因为普通的 `cargo test` / `cargo build` 会把同一个二进制
**不带** `mpv-runtime` 重新编译并悄悄覆盖掉它，之后启动就会看到
"Playback runtime is disabled in this build"，看起来完全像一次回归。
:::

## 打包

```powershell
pwsh -NoProfile -File scripts/fetch-runtime.ps1 -Platform windows-x64
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime
```

产物在 `dist/YoYoVideo-windows-x64/` 与 `dist/YoYoVideo-windows-x64.zip`。

验证与冒烟：

```powershell
pwsh -NoProfile -File scripts/verify-package.ps1 -Platform windows-x64 -RequireRuntime
pwsh -NoProfile -File scripts/smoke-package.ps1 -Platform windows-x64 -RequireRuntime
```

`smoke-package.ps1` 会生成一段 WAV，用打包出来的播放内核真的解码一次，并等待时长、进度、轨道
事件出现——**解码不了的产物到不了发行页**。

可选的 NSIS 安装包：

```powershell
pwsh -NoProfile -File scripts/build-installer.ps1 -PackageDir dist/YoYoVideo-windows-x64 -OutputPath dist/YoYoVideo-windows-x64-setup.exe
```

## 更新播放内核

播放内核的版本和校验和写在 `runtime/manifest.toml` 里。升级时改 `source_url` / `sha256` /
`version` 三个字段，然后重新跑一次 fetch + package + smoke。
`version` 参与缓存文件名，所以改版本号会让脚本重新下载而不是复用旧缓存。


## 图标

应用图标是**生成**的，不是手工导出的位图：

```powershell
node scripts/generate-icons.mjs
```

它会用同一套几何（圆角方块 + 播放三角，与 `docs-site/docs/public/logo.svg` 同源）渲染出
Windows 的 `.ico`（7 个尺寸）、Linux 用的 PNG，以及 Slint 窗口图标用的 PNG。产物已提交到仓库，
改了形状之后重跑这个脚本即可——没有依赖任何图像工具，PNG 和 ICO 都是脚本自己编码的。