# 自动更新与发布

## 支持范围

更新后端是 Velopack **1.2.161**（Rust SDK 与 vpk 固定同版本），不是 Tauri 插件。
新发布从 0.0.1 开始，不做旧 NSIS 安装的原地迁移。普通开发构建、旧 ZIP 和 .deb
只提供手动更新入口。Windows 安装版、macOS .app、Linux AppImage 使用独立稳定 channel：

- `stable-windows-x64`
- `stable-macos-aarch64`
- `stable-macos-x86_64`
- `stable-linux-x64`

仅完整包，不生成 delta。更新来源固定为 `ijry/YoYoVideo` 的 GitHub Releases。
自动检查可关闭，手动检查不受自动检查节流限制；本地播放无需网络，无遥测。
下载完成不等于安装：用户确认后才保存状态、检查占用、退出并启动更新助手。
启动自动应用缓存已禁用。“稍后”不会在下次启动时变成自动安装。

## 三种不同的签名

1. **更新认证**：现有 minisign/Tauri 格式密钥签署各平台的
   `yoyovideo-update.<platform>.json`，对应 `.json.sig`。客户端在解析清单和安装前验签，
   按清单校验 full nupkg 的 SHA-256/大小。原生 feed 不是独立的可信来源。
2. **macOS 系统签名**：目前只有 `--signAppIdentity -` 的 ad-hoc 签名，**无 Apple 公证**。
   不能消除 Gatekeeper 提示，也不能称作 Apple Developer 签名。
3. **Windows Authenticode**：目前没有 Windows 受信任代码签名证书。
   更新认证密钥不能消除 SmartScreen/未知发布者提示。

发布包内不携带私钥。Pinned public key 位于
`apps/yoyovideo-desktop/assets/updater.pub`，公钥 ID 为 `DD3DF8EED25DA3E5`。
GitHub Variable 必须与客户端公钥一致，否则构建失败。

## 密钥保存

维护者本机密钥位于 `D:/Repos/xyito/config/yoyovideo`，不在仓库内。
加密私钥与密码需要分别安全备份，文件夹保持仅本人、SYSTEM、Administrators 可访问。
不要把密码或私钥复制进脚本、提交、日志、构建缓存或 Release 资产。

GitHub repository 配置：

- Secret `YOYOVIDEO_UPDATER_PRIVATE_KEY`
- Secret `YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD`
- Variable `YOYOVIDEO_UPDATER_PUBLIC_KEY`

只有 release 的独立签名步骤注入两个 Secret；编译、播放器探测、vpk、PR 构建和签名验证
不接收它们。公共测试使用一次性测试密钥，测试包不得上传 stable Release。
更换公钥需要独立迁移设计，不能仅替换服务器密钥后继续发布。

## 原生包装

需要 PowerShell 7、.NET 8 和匹配架构的 Rust/libmpv 工具链。固定安装工具：

```powershell
dotnet tool install vpk --version 1.2.161 --tool-path .cache/tools/vpk
pwsh -NoProfile -File scripts/fetch-runtime.ps1 -Platform windows-x64
pwsh -NoProfile -File scripts/package.ps1 -Platform windows-x64 -Configuration release -RequireRuntime -ReleaseVersion 0.0.1 -StageOnly
pwsh -NoProfile -File scripts/package-velopack.ps1 -Platform windows-x64 -Version 0.0.1 -PackageDir dist/YoYoVideo-windows-x64 -OutputDir dist/velopack/windows-x64 -PrepareOnly
```

`-PrepareOnly` 只生成/校验未签名产物，**不代表可发布**；正式校验不加 `-BeforeSigning`，
必须通过 Rust 验签器。默认打包模式需要签名凭据以及已构建的 `yoyo-update-sign`。
输出目录必须为空。脚本在 `.cache/velopack-*` 保留隔离诊断，不安装到用户目录。
`--build-info` 探测会检查真实版本、mpv-runtime 和 updater，防止误发无播放内核的构建。

### macOS

ARM/Intel 分别原生构建。.app 的二进制及 dylib 放 `Contents/MacOS`，说明与许可证放
`Contents/Resources`。打包时使用 ad-hoc 签名；原生验证运行 `codesign --verify` 和
`otool`，拒绝残留 Homebrew/runner 绝对依赖。不使用假证书，也不声称已公证。

### Linux

AppImage **构建基线固定为 Ubuntu 22.04 x86_64**，不是 ubuntu-latest。
Jammy 提供 libmpv1/.so.1，而较新系统通常提供 libmpv2/.so.2；按实际 SONAME 链接/收集，
绝不把 .so.1 重命名伪装成 .so.2。原生 CI 必须验证真实解码。

`stage-appimage-runtime.ps1` 从 ELF 直接依赖和明确的 dlopen 库收集闭包，复制分发版权通知、
common-licenses 与文件哈希，记录于 `usr/bin/LICENSES/appimage-runtime.json`。
AppRun 为包内库设置 LD_LIBRARY_PATH，完整 nupkg 包含生成的同一份 AppImage。
glibc、动态加载器、C++ 宿主 ABI 与图形驱动/dispatch 不打包。仍需系统字体/桌面、
图形驱动、音频服务，部分可选音频/硬件插件来自宿主；不能声称在任意 Linux 系统运行。

.deb 保持系统依赖方式，由包管理器更新，不套用 AppImage 的库收集逻辑。

## 发布门禁

- `updater-build.yml`：四平台无私钥构建、解码冒烟、原生包验证，输出未签名资产。
- `updater-smoke.yml`：PR/push/手动运行上述包装验证，**不是安装升级端到端测试**。
- `release.yml`：只接受版本与 Cargo.toml 一致的稳定 annotated tag，构建精确 commit。
  四平台完成后，独立构建签名器、无覆盖合并、校验原生产物，再签署全部清单。
- `verify-updater-release.mjs`：检查精确四平台集合、签名、原生 feed 一致性、包内容和哈希；
  未列出的文件/缺失文件/混合版本/重复资源名全部拒绝。
- 发布顺序：完整本地验证 → 新建 draft → 上传 → 下载 draft 全资产 → 再次验签及内容校验 →
  与上传前所有文件的哈希比较 → 再检查 tag commit → publish。

**已有同 tag 的公开 Release 或 draft 一律拒绝，不使用 clobber。**
重新发布旧 0.0.1 必须由维护者另行明确操作；工作流不会自行删除旧 Release 或重打 tag。
上传/下载/复验失败时只留下 draft，必须调查后再处理，不能直接手动公开未验证产物。

## 诊断与恢复

更新窗口显示网络、签名、占用与安装失败信息；运行时警告写入
`AppPaths.data_dir/logs/yoyovideo.log`（路径由 ProjectDirs 按 OS 计算）。无法发现数据目录
的启动错误写当前工作目录 `logs/yoyovideo.log`。缓存位于 `AppPaths.cache_dir/updates`。
反馈时附错误文本、应用版本、平台及相关助手日志，勿附私钥、token 或媒体隐私。

- 网络失败：稍后手动检查，或从固定 Release 页手动下载；播放不受影响。
- 验签/哈希失败：停止安装，保留诊断并重新检查可信发布；不要绕过签名校验。
- 文件占用：先正常关闭同安装目录的其他实例，再重试。预检查不能消除所有占用竞态。
- 更新后异常：从可信发布页重新安装正确平台包；不要删除用户数据目录来“修复升级”。

## 当前验证状态（2026-10-08）

本机 Windows 已运行真实 vpk 打包、一次性密钥签名/独立验签、篡改拒绝；**没有运行安装器**。
Windows 上的跨平台 ZIP/ELF/Mach-O fixtures 只证明格式契约，不代表 macOS/Linux 原生通过。
本次四目标 GitHub CI 尚未运行；实际 0.0.1 → 0.0.2 下载安装/重启/状态保留仍待 Task 8。
0.0.1 尚未重新发布。后续完成时分别更新这些状态，不把“打包成功”当作“升级成功”。
