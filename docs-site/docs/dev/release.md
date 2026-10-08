---
title: 发布流程
description: 四平台 Velopack 构建、更新认证和 draft 复验门禁。
---

# 发布流程

版本号只来自根目录 `Cargo.toml` 的 `[workspace.package]`，各 crate 使用 `version.workspace = true`。
发布只接受与版本一致的稳定 `vX.Y.Z` **annotated tag**；发行说明读取 tag 的 annotation，而不是其指向的 commit message。

## 流程

1. 提交经过测试的版本，准备非空 tag annotation，再推送对应 tag。工作流把 tag 解析成精确 commit。
2. `updater-build.yml` 构建 Windows x64、macOS ARM64、macOS Intel、Linux x64。
   macOS 两种架构分别原生构建；Linux AppImage 固定 Ubuntu 22.04 基线。
3. 各平台拉取 libmpv、构建真实播放内核、解码冒烟、用固定 **vpk 1.2.161** 包装并原生验证。
   这里没有生产签名私钥。正式 Windows 安装包不再使用旧 NSIS 流程。
4. 四个平台包装、Windows 首次 Setup 安装，以及 Windows/macOS/Linux 真实升级均成功后，
   发布 job 无覆盖合并资源并校验，独立签名步骤签署四份更新清单。
5. 完整验签与资产校验 → 上传到新 draft → 下载全部 draft 资源 → 再次验签、校验包内容及所有文件哈希 →
   再核对 tag commit → 公开 Release。任一步失败都不会公开。
6. 成功后通知文档站重新部署。

资产表和 SHA-256 从实际产物生成，不手写不存在的下载链接。
只发布完整更新包，不生成 delta。每个平台有独立稳定 channel。

## 密钥与系统签名

GitHub Secrets 是 `YOYOVIDEO_UPDATER_PRIVATE_KEY` 和 `YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD`；
Variable `YOYOVIDEO_UPDATER_PUBLIC_KEY` 必须与客户端 `assets/updater.pub` 一致。
仅签名步骤注入私钥。PR/普通构建不使用生产密钥。

更新认证不等于系统代码签名：macOS 使用 ad-hoc 签名，无 Apple 公证；Windows 无受信任 Authenticode 证书。
不要把这些限制描述成“已消除系统拦截”。

## 重跑与重新发布

可以手动触发 `release.yml` 并指定现有稳定 tag，但**已有同 tag 的公开 Release 或 draft 一律拒绝**。
不使用 `allowUpdates`/clobber，不自动删除 Release 或移动 tag。
失败留下的 draft 需要先调查，不能直接公开。旧 0.0.1 的重新发布必须另行明确处理。

## 验收状态

2026-10-08，提交 `81d7f06` 的[完整原生 CI](https://github.com/ijry/YoYoVideo/actions/runs/37736646064) **9/9 通过**，
[普通 CI](https://github.com/ijry/YoYoVideo/actions/runs/37736645755) 通过。macOS 两种架构各连续 3 次升级必过，
Linux 四种组合全部通过；测试未使用生产私钥。

随后经维护者单独授权，已重发 [v0.0.1](https://github.com/ijry/YoYoVideo/releases/tag/v0.0.1)（源码 `4c04726`）。
[正式发布流水线](https://github.com/ijry/YoYoVideo/actions/runs/37750925601) **10/10 通过**；
29 个公开附件下载校验、四份正式更新签名和匿名更新入口均已核对。旧版本附件与 tag 已备份。

`updater-smoke.yml` 同时覆盖四平台原生包装/解码、Windows Setup 首次安装/快捷方式/卸载，
以及真实 `0.0.1 → 0.0.2` 升级：Windows portable-layout、macOS ARM64/Intel、
Linux 22.04/24.04 × FUSE/解压运行（容器无系统 libmpv）。
每个升级案例检查坏签名/坏包/缓存篡改拒绝、稍后不安装、占用保护、新进程版本和历史恢复。
另外用一次性密钥签署四平台实际产物，验证完整发布集合；测试私钥和 QA 包不上传 Release。
正式发布依赖同样的包装与 `updater-upgrade-windows.yml`、`updater-upgrade-unix.yml` 门禁。
精确命令、密钥备份和当前验收状态见[更新机制文档](https://github.com/ijry/YoYoVideo/blob/main/docs/development/updater.md)。
