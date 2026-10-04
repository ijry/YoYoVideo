---
title: 发布流程
description: YoYoVideo 的版本号规则、tag 触发流程与发行资产构成。
---

# 发布流程

## 版本号只有一个来源

版本号写在仓库根目录 `Cargo.toml` 的 `[workspace.package]` 里：

```toml
[workspace.package]
version = "0.0.1"
```

三个 crate 都用 `version.workspace = true` 继承它，没有第二处可以改。
`.github/workflows/release.yml` 会比对 tag 与这个值，不一致直接拒绝发布——
所以"代码里是 0.0.1、tag 却打了 0.0.2"这种漂移不会发生。

## 发一个版本

```powershell
# 1. 改 Cargo.toml 里的 version，并写好 changelog
# 2. 提交并推送
git commit -am "release: 0.0.1"
git push origin main

# 3. 打 tag —— commit message 就是这一版的 changelog
git tag -a v0.0.1 -m @"
## 0.0.1

首个公开版本。

- 多画面批量播放
- 内嵌 libmpv 运行时，无需预装解码器
- Windows x64 便携包与安装包
"@
git push origin v0.0.1
```

打 tag 的那条命令会触发 `.github/workflows/release.yml`。

::: warning tag commit 的 message 就是 changelog
发行页正文直接取 tag commit 的完整 message。GitHub 自动生成的 release notes 对一个 release tag
只会给一条 compare 链接，那不叫 changelog。发布前请确认 message 不是空的。
:::

## 流水线做了什么

1. **prepare** — 校验 tag 格式、比对 `Cargo.toml` 版本号、读取 tag commit 的 message 作为
   changelog、确认哪些平台可以发布。
2. **build**（`windows-latest`）— 拉取并校验播放内核 → 打包 → **跑播放冒烟测试** →
   打 NSIS 安装包 → 上传产物。
3. **publish**（`ubuntu-latest`）— 下载产物，用 `scripts/create-release-body.mjs` 生成发行页正文
   （changelog + 资产表 + 每个资产的 SHA-256），然后创建或更新 Release。
4. 顺带 dispatch 一次 `docs.yml`，让文档站重新部署。

资产列表是**从实际产物生成的**，不是手写的——某个平台的构建失败时，发行页不会出现指向
不存在文件的下载链接。

## 为什么只能发 Windows

`runtime/manifest.toml` 里每个平台有一条记录，带 `available` 字段。macOS 与 Linux 目前是
`available = false`，原因写在各自的 `notes` 里：macOS 没有经过审核的通用架构 libmpv
（上游只发 Windows 构建，Homebrew 只有分架构 bottle，硬凑"universal"是不诚实的），
Linux 则需要打包 libmpv 的完整依赖闭包。

`prepare` 这一步会先检查这一点，不会在矩阵跑到一半才发现某平台没法构建。

## 补一个新平台

1. 在 `runtime/manifest.toml` 对应条目里填上真实的 `source_url`、`sha256`、`version`，
   把 `available` 改成 `true`。
2. 在 `scripts/fetch-runtime.ps1` 里为该平台补上归一化逻辑（Windows 那段把 DLL 改名并重建
   导入库；其他平台直接把归档内容放进 `lib/`）。
3. 跑一次本地 `fetch` → `package` → `smoke-package`，确认播放真的能工作。
4. 在 `release.yml` 的矩阵里加上该平台，并把 `prepare` 里的平台列表一起改掉。

## 手动重发

某个平台构建挂了、想用同一个 tag 重跑：

```powershell
gh workflow run release.yml --ref v0.0.1 -f tag=v0.0.1
```

`allowUpdates: true`，所以重跑会覆盖同一个 Release 里的产物。
