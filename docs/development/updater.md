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

- `updater-build.yml`：四平台无私钥构建、解码冒烟、原生包验证，输出未签名资产；
  Windows job 还在一次性 GitHub-hosted runner 上执行首次 Setup 安装和卸载验收。
- `updater-upgrade-windows.yml`、`updater-upgrade-unix.yml`：运行真实两个编译版本的
  验签、下载、拒绝篡改、稍后、占用保护、正常退出、原生替换、重启和历史恢复。
- `updater-smoke.yml`：PR/push/手动运行上述包装、安装和升级回归；另用一次性测试密钥
  签署本次构建的四平台资产，通过与正式发布相同的完整集合验证器。
- `release.yml`：只接受版本与 Cargo.toml 一致的稳定 annotated tag，构建精确 commit。
  四平台包装、Windows Setup、Windows/macOS/Linux 真实升级全部成功后，
  独立构建签名器、无覆盖合并、校验原生产物，再签署全部清单。
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

## 真实跨平台升级回归

```powershell
pwsh -NoProfile -File scripts/test-velopack-upgrade.ps1 -Platform windows-x64 -FromVersion 0.0.1 -ToVersion 0.0.2
# 以下命令分别在对应架构的 macOS 原生运行
pwsh -NoProfile -File scripts/test-velopack-upgrade.ps1 -Platform macos-aarch64
pwsh -NoProfile -File scripts/test-velopack-upgrade.ps1 -Platform macos-x86_64
```

脚本在 `.cache/updater-upgrade-build-*` 中复制源码，分别编译两个真实版本，生成一次性密钥，
用真实 vpk 包装/签名。`-BuildOnly` 只准备产物；`-BuildRoot <已生成目录>` 重跑既有二进制，
Rust 实现变更后应重新构建，不要用旧目录当作新代码的证据。

测试 feature 是桌面的 `updater-qa` 与核心的 `qa-fixture`，均非默认。
只有这些构建接受带 `TEST-ONLY` 标记的 `YOYOVIDEO_UPDATER_QA_ROOT`，其中存放测试公钥、
本地更新源、控制请求、独立用户目录和事件记录；SDK 包缓存也限定在该次运行的
`sdk-packages` 中，不污染 macOS 用户缓存或 Linux `/var/tmp/velopack`。正式构建不接受该环境入口。
QA 请求绑定 PID 和递增序号，打开的媒体必须位于测试目录内；实际执行复用 UI 回调和正式验签/下载/安装状态机。

QA 包使用独立标题 `YoYoVideo QA ONLY`，build-info 标记 `updater_qa=true`，禁用快捷方式。
只能使用 `-PrepareOnly -QaFixture` 包装。默认生产包装和发布验证均拒绝这些包，
即使 Linux 校验不展开 AppImage，也会在外层 nuspec 标题检查处拒绝。

Windows 测试使用 SDK 认可的 `.portable` 布局和真实 `Update.exe`，不运行 Setup，
因此不写入真实用户的卸载注册项/快捷方式。测试覆盖：

- 实际解码 WAV 并保存播放历史；
- 错误签名、损坏下载、缓存篡改都不安装；
- 选择稍后、关闭并重启后仍为旧版，等待超过自动检查延迟也不偷偷安装；
- 同目录另一实例存在时拒绝安装，不杀掉它；
- 真正退出旧进程、由更新助手替换并启动新版，新 PID 报告编译版本 0.0.2；
- 新版不自动播放，保留自动检查偏好及历史，并在 8 秒内恢复到至少 15 秒的历史位置，
  不能靠从头播放来冒充恢复；
- 同版本检查为最新，最后确认所有播放器正常终止（Windows 检查内核进程对象；Unix 同时核对正常关闭事件）。

`events.jsonl`、`SUCCESS.json` 和日志保留在该次 `run-*` 目录；成功以新版进程的报告为准，
不是以安装助手返回 0 为准。失败清理先核对已打开进程的实际映像路径，只处理隔离安装目录，
不清理/终止用户正常安装的播放器。CI 只上传事件/日志，不上传测试私钥或 QA 包。

### Windows 退出修复

原来的 Winit 线程局部事件处理器可能把播放后端留到进程/TLS 清理时才释放，
此时 libmpv 的其他线程已经终止，导致进程停留在退出中。`HasExited`/退出码 0
不足以证明进程内核对象完成终止。现在在保存状态后显式释放 Windows 播放后端，
并让 Windows 网格先释放 mpv session、再释放嵌入 HWND；macOS 的 render-context 优先顺序不变。
占用预检查使用零超时内核等待核对存活状态，无法检查的潜在冲突仍拒绝安装。

### macOS 与 Linux

macOS 在 ARM64/Intel 原生 runner 上，通过真实 `UpdateMac` 替换 .app 并由 LaunchServices
重启。验证与目标架构匹配的 Mach-O、依赖和 ad-hoc 签名；更新助手本身允许是包含目标架构的
universal binary。每个架构连续执行三次完整升级，任意一次失败都会令 job 失败，
不是重试直到成功。超时保留 QA 启动阶段、限定到测试安装路径的进程栈和相关崩溃/系统日志。

原生 Intel CI 曾捕获更新后新进程的 AppKit 回调重入崩溃：Slint display-link 定时器调整
视频窗口时同步产生 Moved/Resized 等事件，重入仍持有借用的运行状态。现在将 macOS 的
视频窗口位置、尺寸和可见性变更排入事件循环，不丢弃窗口事件，也不靠延长超时或禁止 App Nap
掩盖问题。排队操作仅持有弱引用，不延长已关闭视频窗口的生命周期。

Linux 从 Ubuntu 22.04 原生构建 AppImage，再在 **22.04 和 24.04** 两套干净容器中，
分别执行 `extract` 与 `fuse` 四种组合。容器无系统 libmpv，只有声明的宿主字体、桌面和
图形 ABI 库。FUSE 案例检查真实挂载而非悄悄降级为解压；每种组合都执行完整升级负例和历史恢复。
复现入口及容器参数见 `updater-upgrade-unix.yml` 与 `scripts/qa-linux/`。
Docker 使用 `--init` 并设置总超时，避免 Xvfb 的就绪信号在 PID 1 被吞掉。

Linux 进程占用检测明确排除 sysinfo 的 task/thread 条目，防止把自身工作线程误判为另一个实例。
AppImage 的 build-info 探测允许运行时通知行，但必须且只能包含一份有效应用报告；
错误通知不能伪装成成功报告。

### Windows Setup 首次安装

`scripts/test-velopack-setup.ps1` 只允许在一次性 GitHub-hosted Windows runner 运行，
拒绝开发者本机；发现已有安装注册项或快捷方式时也拒绝覆盖。它用真实 Setup 静默安装，
核对 HKCU 的名称/版本/安装路径/卸载命令，以及桌面和开始菜单快捷方式指向
`current/yoyovideo-desktop.exe`，运行已安装程序的 build-info，随后实际卸载，
检查注册项、快捷方式和程序移除、用户数据哨兵保留。证据为 `setup-windows-diagnostics`。

## 已完成的原生验收（2026-10-08）

实现提交：`81d7f062a412ee04687f10ea0c53b03e385f6759`。
[完整原生烟测 37736646064](https://github.com/ijry/YoYoVideo/actions/runs/37736646064)
**9/9 作业成功**；[普通 CI 37736645755](https://github.com/ijry/YoYoVideo/actions/runs/37736645755) 也已通过。
不是仅生成工作流或只做 Windows 本机模拟。

| 验收项目 | 实际结果 |
| --- | --- |
| Windows x64 | 真实 `0.0.1 → 0.0.2` portable-layout 替换、重启、历史恢复通过；另在干净 runner 上通过 Setup 首次安装、HKCU/两处快捷方式、卸载及用户数据保留 |
| macOS ARM64 | 原生 .app 替换/LaunchServices 重启，连续 **3/3** 完整升级通过 |
| macOS Intel | 修复窗口回调重入崩溃后，原生 .app 替换/重启连续 **3/3** 通过 |
| Linux x64 | 无系统 libmpv 的 Ubuntu 22.04/24.04 × FUSE/解压运行，**4/4** 完整升级通过，FUSE 检查真实挂载 |
| 四平台发布集合 | 全部原生生产包构建/解码/内容检查通过；一次性密钥签署四份清单，再由正式发布验证器独立验签及校验完整资产集合 |

共核对 **11 次真实升级**及独立 Windows Setup 的 `SUCCESS.json`。每次升级覆盖坏签名、坏包、
缓存篡改、稍后不安装、另一实例占用保护，要求新 PID 报告真实编译版本 0.0.2，且在 8 秒内
恢复至少 15 秒的历史位置。macOS 的三次均为必过用例，不是失败重试取一次成功。
事件和诊断保存在上述 workflow 的 `windows-upgrade-diagnostics`、
`upgrade-<platform>-diagnostics`、`setup-windows-diagnostics` 产物中；本机复核汇总位于
`.cache/native-acceptance-81d7f06/verified-summary.json`。测试私钥和 QA 包不上传。

本机回归（Windows，同一实现提交）：

- `cargo test --workspace -j 2`：**308 通过、0 失败、1 项既有忽略**。
- `cargo test -p yoyo-updater --features qa-fixture -j 2`：45 通过；
  桌面 `updater-qa` 专用控制/事件测试 3 通过，`mpv-runtime` 视频宿主契约 5 通过。
- `node --test scripts/test-updater-release.mjs`：10 通过；
  `scripts/test-velopack-package.ps1`：20 通过（含真实 Windows build-info 探测）。
- AppImage 策略/两种 SONAME staging、QA 布局/事件解析、真实 Windows vpk 一次性签名集成、
  fmt、更新核心 Clippy、actionlint、文档站 4 项测试及构建通过。
- 非 QA 的 runtime-enabled 程序已重建、报告非 QA，二进制不含 QA 环境入口。
  GitHub 公钥 Variable 与 pinned key 一致，Secret 仅核对名称而未读取值。

## 0.0.1 正式重发（2026-10-08）

维护者另行授权后，已于 **17:24（UTC+8）** 重新发布
[`v0.0.1`](https://github.com/ijry/YoYoVideo/releases/tag/v0.0.1)，`latest` 指向此版本。
发布源码固定为 `4c047264385b0630cbd89eb601a964e11c611d1c`，Release ID 为 `406651136`。

- [正式 Release 流水线 37750925601](https://github.com/ijry/YoYoVideo/actions/runs/37750925601)
  **10/10 作业通过**，包含四平台构建、Windows Setup、真实升级、正式签名、draft 下载复验及公开发布。
- 先保留旧下载，完成新包构建、正式签名和验签后，才删除已备份的旧 Release；
  同 tag 保护按预期拒绝首次发布尝试，随后仅重跑发布作业，复用已通过的原生构建/升级结果。
- 新版本共 **29 个附件**。从公开 Release 重新下载后，29 个 SHA-256 均与 GitHub 元数据一致，
  四个平台的更新清单均用该 tag 内的正式公钥独立验签通过。Windows 的匿名 `latest/download`
  清单和签名入口也已核对，与已验签的文件一致。
- 原 Release 的 5 个附件、完整元数据及旧附注 tag 已备份到维护者本地
  `.cache/republish-0.0.1-*`；没有把密钥或 QA 包上传到公开 Release，0.0.2 仍仅用于测试。
- 发布 tag 解析使用独立校验引用，避免 checkout 将同名 tag 绑定到提交对象时发生冲突；
  实际执行 tag 校验脚本的 Git 回归已纳入 `test-updater-release.mjs`，该测试集现为 **14 项通过**。


macOS 仍为 ad-hoc、未公证；Windows 仍无受信任 Authenticode 证书。原生回归验证真实 WAV
解码及升级/状态生命周期，不替代所有 GPU、视频、Wayland、多宫格兼容性测试，
也不证明 Gatekeeper/SmartScreen 提示已消失。
