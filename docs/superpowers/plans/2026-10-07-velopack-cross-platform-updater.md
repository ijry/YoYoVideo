# YoYoVideo 三平台自动升级 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 交付带更新认证、正常退出安装和四个构建目标的 Velopack 自动升级，不把密钥或规划完成误报为功能完成。

**Architecture:** 独立 yoyo-updater crate 提供签名协议、带验证的 UpdateSource、后台服务和状态策略；独立 yoyo-update-sign 发布工具使用现有 minisign 密钥。Slint 只负责交互与生命周期，vpk 负责平台版本包及实际替换。

**Tech Stack:** Rust、Slint 1.17、Velopack/vpk 1.2.161、minisign-verify 0.3、minisign 0.10、SHA-256、PowerShell、GitHub Actions、.NET 8 SDK（仅构建工具）。

**Spec:** docs/superpowers/specs/2026-10-07-velopack-cross-platform-updater-design.md

## Global Constraints

- 应用版本起点 0.0.1；不做旧版迁移；不在实施测试中修改远端 tag 或覆盖公开 Release。
- targets: windows-x64、macos-aarch64、macos-x86_64、linux-x64；channel 固定为 stable- 加 target。仅 full 包，无 delta，无降级或同版本自动安装。
- SDK 与 vpk 固定为 1.2.161；保留 workspace rust-version = 1.85 声明，发现新增依赖冲突时记录并处理，不能静默改变要求。
- macOS --signAppIdentity -，不公证；Linux AppImage 自更新，.deb 不自更新。
- 公钥固定进应用并与 GitHub Variable 比较；私钥和密码不进仓库、命令行参数、日志或产物。
- 元数据上限 8 MiB、包上限 2 GiB；连接超时 10 秒、元数据总超时 30 秒、包下载总超时 30 分钟。
- 启动延迟 10 秒自动检查，自动检查最多每 24 小时一次；用户确认下载，确认后才重启安装。
- 关闭 SDK 启动自动应用；安装使用 wait_exit_then_apply_updates，保存与析构不能被 process::exit 跳过。
- 保留未提交的 docs/testing/manual-smoke-checklist.md、installer/windows/*、scripts/build-installer.ps1、scripts/test-windows-installer.ps1、scripts/fixtures/ 等既有工作；禁止 git add -A。
- 每个实现步骤先写能暴露缺失行为的测试并确认 RED，再实现并确认 GREEN。外部 OS/网络边界可替身，签名、版本、文件和状态决策用真实代码。

## 文件归属和依赖顺序

1. 协议：crates/yoyo-updater/{Cargo.toml,src/lib.rs,src/error.rs,src/manifest.rs,src/policy.rs,tests/manifest_contract.rs,tests/policy_contract.rs}。
2. 发布签名：crates/yoyo-update-sign/{Cargo.toml,src/lib.rs,src/main.rs,tests/sign_contract.rs}，apps/yoyovideo-desktop/assets/updater.pub。
3. 服务：crates/yoyo-updater/src/{source.rs,service.rs,preferences.rs}，相应 tests/source_contract.rs、service_contract.rs、preferences_contract.rs。
4. UI：apps/yoyovideo-desktop/ui/update-window.slint、main-window.slint，tests/update_window_contract.rs。
5. 播放器接入：apps/yoyovideo-desktop/src/{main.rs,lib.rs,app.rs,update_runtime.rs}、Cargo.toml、tests/update_runtime_contract.rs。
6. 打包：scripts/{package-velopack.ps1,verify-velopack-package.ps1,test-velopack-package.ps1,stage-appimage-runtime.ps1}；scripts/package.ps1 只增加独立 staging 能力，避免改写原安装器脚本。
7. CI：.github/workflows/{release.yml,ci.yml,updater-smoke.yml}，scripts/test-updater-release.mjs、scripts/verify-updater-release.mjs、README.md、README-EN.md、docs/development/updater.md。
8. 原生验证：scripts/test-velopack-upgrade.ps1、apps/yoyovideo-desktop/src/update_qa.rs、tests/fixtures/updater/（仅公开测试密钥/数据）。

Cargo.toml、Cargo.lock 的写入由整合者统一完成。协议/服务、UI、打包可以分开写入；播放器接线等待前三者的接口稳定。

## Task 1: 可验证的签名清单与平台策略

**Files:** 新建协议文件及测试，修改 workspace Cargo.toml/Cargo.lock。

**Interfaces:**

```rust
pub enum Platform { WindowsX64, MacosArm64, MacosX64, LinuxX64 }
// as_str 返回规格中的 target；channel 返回 stable-<target>；current 返回 Option<Platform>。
pub struct UpdateManifest {
    pub schema_version: u32, pub app_id: String, pub platform: String,
    pub channel: String, pub version: String, pub release_tag: String,
    pub feed: velopack::VelopackAssetFeed,
}
pub struct VerifiedManifest {
    manifest: UpdateManifest, raw: Vec<u8>, signature: String,
}
// verify(raw, signature_base64, public_key_base64, platform) -> Result<VerifiedManifest, UpdateError>
// manifest() -> &UpdateManifest; asset() -> &velopack::VelopackAsset
// is_newer_than(current: &semver::Version) -> bool
// verify_package(path: &std::path::Path) -> Result<(), UpdateError>
```

UpdateError 分别表示签名、清单、平台、版本、哈希、大小、网络、持久化、安装和不支持环境错误；Display 不包含私钥/密码/响应原文。

- [ ] **1.1 RED：** 添加 workspace member 和 crate 测试所需最小 Cargo 结构，不写协议实现。真实 minisign 测试辅助函数按下列代码生成测试签名：

```rust
use base64::{Engine, engine::general_purpose::STANDARD};
fn signed(raw: &[u8]) -> (String, String) {
    let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
    let key = STANDARD.encode(pair.pk.to_box().unwrap().into_string());
    let sig = minisign::sign(Some(&pair.pk), &pair.sk, raw, Some("yoyovideo test"), None).unwrap();
    (key, STANDARD.encode(sig.to_string()))
}
```

测试正文使用 version=0.0.2、release_tag=v0.0.2、windows-x64、stable-windows-x64、PackageId=YoYoVideo、Type=Full、单个文件 YoYoVideo-0.0.2-full.nupkg，包内容为 abc，Size=3。SHA256 固定 ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad，SHA1 固定 a9993e364706816aba3e25717850c26c9cd0d89d。断言真实 verify 成功；修改正文/可信注释/公钥失败。另对重新签名但上下文错误、跨平台、缺 SHA256、delta、多 full、坏文件名、过大正文逐项拒绝。

```powershell
cargo test -p yoyo-updater --test manifest_contract
```

预期首次失败是缺少协议能力；修复测试编译问题后，确认拒绝测试真的失败，不能把编译错误当完成 RED。

- [ ] **1.2 GREEN：** 实现 base64 外层解码、UTF-8 边界、minisign-verify 的 PublicKey/Signature::decode 与 verify(raw, &signature, false)，再解析 serde JSON。严格检查规格所有上下文字段和 full 包；用 semver 而非字符串比较版本。构建 VerifiedManifest 时缓存 raw/signature，以便重启后重新验证，字段不对外开放修改。

```rust
use std::io::{Read, BufReader};
use sha2::{Digest, Sha256};
let mut input = BufReader::new(std::fs::File::open(path)?);
let mut hash = Sha256::new();
let mut total = 0_u64;
let mut buffer = [0_u8; 64 * 1024];
loop {
    let read = input.read(&mut buffer)?;
    if read == 0 { break; }
    total += read as u64;
    hash.update(&buffer[..read]);
}
```

把 total 与签名 Size、最终摘要与签名 SHA256 比较；不先把大包整体读进内存。

- [ ] **1.3 测试策略：** 添加 as_str/channel/current 映射、0.0.1→0.0.2、0.0.2→0.0.2、0.0.3→0.0.2、预发布排除和所有版本字段不一致测试。
- [ ] **1.4 验证并提交：**

```powershell
cargo test -p yoyo-updater
cargo fmt --check
git add -- Cargo.toml Cargo.lock crates/yoyo-updater
git commit -m "feat: authenticate updater manifests and packages"
```

## Task 2: 发布签名工具和固定公钥

**Consumes:** Task 1 的 DTO、VerifiedManifest。
**Produces:** yoyo-update-sign sign / verify 两个命令；同时提供库函数供真实测试调用。

命令契约：

```powershell
cargo run -p yoyo-update-sign -- sign --feed releases.stable-windows-x64.json --platform windows-x64 --version 0.0.1 --assets-dir dist/velopack/windows-x64 --output dist/velopack/windows-x64/yoyovideo-update.windows-x64.json --public-key apps/yoyovideo-desktop/assets/updater.pub
cargo run -p yoyo-update-sign -- verify --manifest dist/velopack/windows-x64/yoyovideo-update.windows-x64.json --platform windows-x64 --assets-dir dist/velopack/windows-x64 --public-key apps/yoyovideo-desktop/assets/updater.pub
```

sign 从环境 YOYOVIDEO_UPDATER_PRIVATE_KEY 与 YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD 读取现有加密格式，不接受命令行私钥参数。输出 JSON 和同名 .sig；verify 不需要私钥。

- [ ] **2.1 RED：** tests/sign_contract.rs 生成一次性测试密钥，在临时目录用 abc fixture 构造原生 feed。调用库 sign 后，用 Task 1 独立验证签名和包；错误密码、与输入 feed 不符的文件、错误平台/public key、缺文件都必须失败且不留下可发布 JSON。再用 std::process::Command 运行二进制验证退出码，不将测试密钥打印出来。
- [ ] **2.2 GREEN：** 在 CLI 外部先验证所有 full 资源的真实大小/哈希和上下文，然后生成唯一 envelope；按原始 JSON 字节签名，再本地验证，最后原子写入 JSON/.sig。关键加密 API 为：

```rust
let secret = minisign::SecretKeyBox::from_string(&decoded_private_key)?
    .into_secret_key(Some(password))?;
let signature = minisign::sign(
    Some(&public_key), &secret, bytes.as_slice(),
    Some("YoYoVideo authenticated update manifest"), None,
)?;
```

敏感值不进入 Debug 派生结构；错误转换只输出类别。CLI flags 解析拒绝重复、缺值或未知参数。
- [ ] **2.3 固定公钥：** 仅复制 D:/Repos/xyito/config/yoyovideo/updater.key.pub 到 apps/yoyovideo-desktop/assets/updater.pub；比较字节/去除末尾换行后的内容，绝不读取私钥并复制进项目。
- [ ] **2.4 验证提交：**

```powershell
cargo test -p yoyo-update-sign -p yoyo-updater
git add -- Cargo.toml Cargo.lock crates/yoyo-update-sign apps/yoyovideo-desktop/assets/updater.pub
git commit -m "feat: sign and verify release update feeds"
```

## Task 3: 带验签的 UpdateSource、缓存和状态服务

**Files:** yoyo-updater/src/{source,service,preferences,policy}.rs 与对应契约测试。
**Consumes:** Task 1 VerifiedManifest、Platform；Velopack 1.2.161 UpdateSource 和 UpdateManager。
**Produces:**

```rust
pub enum UpdatePhase { Unsupported, Idle, Checking, UpToDate, Available, Downloading, ReadyToInstall, PreparingInstall, Error }
pub struct UpdateSnapshot {
    pub phase: UpdatePhase, pub version: String, pub notes: String,
    pub progress: i16, pub error: String,
}
pub struct UpdatePreferences { pub automatic_check: bool, pub last_checked_at: Option<i64> }
pub enum UpdateCommand { Check, Download, VerifyForInstall, LaunchInstaller, Stop }
pub enum UpdateEvent { Snapshot(UpdateSnapshot), VerifiedForInstall, InstallerStarted }
// ServiceConfig { platform: Platform, public_key: String, cache_dir: PathBuf }
// UpdateService::new(ServiceConfig) -> Result<UpdateService, UpdateError>
// UpdateService::check() -> Result<UpdateSnapshot, UpdateError>
// UpdateService::download(Sender<i16>) -> Result<(), UpdateError>
// UpdateService::verify_for_install() -> Result<(), UpdateError>
// UpdateService::launch_installer() -> Result<(), UpdateError>
// spawn_worker(ServiceConfig) -> (Sender<UpdateCommand>, Receiver<UpdateEvent>)
```

UpdatePreferences::should_check(now: i64, manual: bool) -> bool；load(path: &Path) -> Result<Self, UpdateError>；save(path: &Path) -> Result<(), UpdateError>。

服务内部保留候选 UpdateInfo 和冻结的 VerifiedManifest，不允许 UI 构造或替换候选包。安装未发现 Velopack context 时返回 Unsupported，不在构造窗口时 panic。

- [ ] **3.1 RED：** 测试 source 的实际验签分支；只替换网络 I/O（用私有 Transport trait 提供 signed bytes/包），不替换 verifier。覆盖响应限额、HTTPS 降级、过期快照、错包、不再访问变化后的 latest。测试实际缓存文件先验证成功再篡改，要求 verify_for_install 失败。
- [ ] **3.2 GREEN：** 实现正式 HTTPS 客户端，固定源 https://github.com/ijry/YoYoVideo，给所有请求设定上限、截止时间及 HTTPS 重定向检查。manifest URL 为 releases/latest/download/yoyovideo-update.<platform>.json，包 URL 从已验证 release_tag 与安全文件名构造。实现 UpdateSource 的真实签名：

```rust
fn get_release_feed(
    &self, channel: &str, app: &velopack::bundle::Manifest, staged_user_id: &str,
) -> Result<velopack::VelopackAssetFeed, velopack::Error>;
fn download_release_entry(
    &self, asset: &velopack::VelopackAsset, local_file: &std::path::Path,
    progress: Option<std::sync::mpsc::Sender<i16>>,
) -> Result<(), velopack::Error>;
```

source 内用互斥保护不可变的已验证快照；下载请求的 asset 必须与快照完全匹配。包下载流写入 SDK 指定的 partial 文件，限长、算哈希，错误时只删除本次 partial 文件。额外在服务验证缓存包，不依赖 SDK 是否选择重下载。
- [ ] **3.3 状态与偏好 TDD：** Check/Download 安装请求每次只有一个任务编号，旧任务事件不能覆盖新状态。自动检查开启且距 last_checked_at 至少 86400 秒才触发，手动绕过；持久化到 updater.toml。验证稍后安装的 signed JSON、.sig、包路径在新服务中重新验签/验包，不能只反序列化一个 trusted=true 标志。
- [ ] **3.4 安装调用：** Worker 完成 VerifyForInstall 只发送 VerifiedForInstall，等待 UI 成功保存后另发 LaunchInstaller。应用检查同目录其他实例，检测到时拒绝并提示。启动助手的关键调用为：

```rust
manager.wait_exit_then_apply_updates(
    &info.TargetFullRelease, false, true, Vec::<String>::new(),
)?;
```

不得调用 apply_updates_and_restart、unsafe_apply_updates 或 NoWait。
- [ ] **3.5 验证提交：**

```powershell
cargo test -p yoyo-updater
git add -- crates/yoyo-updater Cargo.lock
git commit -m "feat: implement verified update downloads and install coordination"
```

## Task 4: 独立更新窗口（可与 Task 1–3 并行）

**Files:** ui/update-window.slint、ui/main-window.slint、tests/update_window_contract.rs。不修改 app.rs 或 Cargo manifests。
**Interfaces:** main-window.slint 导出 UpdateWindow 并增加 check_updates_requested() 菜单回调。UpdateWindow 的 public 属性与回调固定为：

```slint
export component UpdateWindow inherits Window {
    in-out property <string> ui_language_code: "zh";
    in-out property <string> current_version;
    in-out property <string> available_version;
    in-out property <string> release_notes;
    in-out property <string> status_message;
    in-out property <int> progress_value;
    in-out property <int> phase_index;
    in-out property <bool> automatic_check: true;
    callback check_requested();
    callback download_requested();
    callback install_requested();
    callback later_requested();
    callback automatic_check_changed(bool);
    callback open_releases_requested();
}
```

phase_index 按 Task 3 UpdatePhase 列出的顺序从 0 起映射。窗口独立，默认约 520×420，适配小屏；不要改变主窗口 800×600 默认值。

- [ ] **4.1 RED：** 按现有 main_window_empty_state_contract.rs 的 headless backend 模式写实际编译窗口测试，验证各 phase 的按钮事件、繁忙时禁用、完成后显示安装/稍后、Unsupported 只引导发布页，以及两种语言。首次先确认缺少接口/动作。
- [ ] **4.2 GREEN：** 导出窗口、添加菜单项，构建深色布局。版本/状态、纯文本可滚动 notes、进度条和动作区明确区分；不会默认触发下载或安装。窗口闭合只发 later_requested。用可访问名称定位测试，不依赖随机布局坐标。
- [ ] **4.3 验证提交：**

```powershell
cargo test -p yoyovideo-desktop --test update_window_contract --test main_window_empty_state_contract
git add -- apps/yoyovideo-desktop/ui apps/yoyovideo-desktop/tests/update_window_contract.rs
git commit -m "feat: add bilingual update window and menu entry"
```

## Task 5: 播放器接线和正常退出

**Files:** desktop Cargo.toml、src/main.rs、src/lib.rs、src/app.rs、src/update_runtime.rs、tests/update_runtime_contract.rs。
**Consumes:** Task 3 worker API、Task 4 UpdateWindow。
**Produces:** UpdateRuntime::attach(&MainWindow, paths: Option<AppPaths>)，open()、poll()；播放器拥有该运行时，不能由视频播放状态决定更新器是否存在。

- [ ] **5.1 RED：** 使用可控 worker 事件与真实偏好临时目录验证后台检查只触发一次、手动入口、关闭弹窗不终止下载、UI 销毁后消息安全丢弃。对退出顺序记录边界调用：VerifyForInstall→保存→LaunchInstaller→正常关窗；保存/启动助手任一失败不得关窗。测试不真正安装程序。
- [ ] **5.2 GREEN：** main 的第一项应用逻辑：

```rust
velopack::VelopackApp::build()
    .set_auto_apply_on_startup(false)
    .run();
```

在 app.rs 创建 UpdateRuntime，把 check_updates_requested 接到 open。独立 timer 定期 drain channel；用户语言变化时同步更新窗口，启动后 10 秒调用受偏好控制的检查。后台任务不得捕获 Rc<DesktopRuntime> 或跨线程操作 Slint window。
- [ ] **5.3 收尾：** 从 app.run() 之后的现有清理提取返回 Result 的更新前保存方法，使用当前播放历史/字幕/标记快照和窗口位置；失败显示诊断且保持播放。VerifiedForInstall 到来后先保存，成功再发送 LaunchInstaller；InstallerStarted 到来后关闭更新、设置与主窗口并退出事件循环。保持 mpv/render drop 顺序，使用 Stop 结束后台 actor；普通关窗仍保留原有收尾语义。
- [ ] **5.4 验证提交：**

```powershell
cargo test -p yoyovideo-desktop --test update_runtime_contract --test history_runtime_contract --test window_state_contract
cargo test --workspace
cargo check -p yoyovideo-desktop --features mpv-runtime
git add -- apps/yoyovideo-desktop/Cargo.toml apps/yoyovideo-desktop/src apps/yoyovideo-desktop/tests/update_runtime_contract.rs Cargo.lock
git commit -m "feat: wire safe player update lifecycle"
```

## Task 6: 四目标 Velopack 包装与验证（UI 并行支线）

**Files:** scripts/package-velopack.ps1、verify-velopack-package.ps1、test-velopack-package.ps1、stage-appimage-runtime.ps1；scripts/package.ps1 仅增加 -StageOnly，不动旧 installer 修改。
**Consumes:** staging 目录、Task 2 signer、固定 vpk 版本。
**Produces:** dist/velopack/<platform> 下安装器/应用包、full nupkg、原生 channel feed、签名 envelope 及 .sig。

命令契约：

```powershell
pwsh -NoProfile -File scripts/package-velopack.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -Version 0.0.1 -OutputDir dist/velopack/windows-x64
pwsh -NoProfile -File scripts/verify-velopack-package.ps1 -Platform windows-x64 -PackageDir dist/YoYoVideo-windows-x64 -ReleaseDir dist/velopack/windows-x64 -Version 0.0.1
```

- [ ] **6.1 RED：** fixture 目录模拟 staging 和 vpk 可执行程序边界；实际运行脚本，断言生成调用的 packId、version、channel、主程序路径、无 delta、macOS ad-hoc 参数，以及缺 libmpv/错误 vpk 版本/缺公钥/缺签名配置时在发布前失败。fixture 只替代 vpk 子进程，不替代本仓库参数与资源验证。
- [ ] **6.2 GREEN：** 查找 dotnet/vpk，验证 1.2.161；缺失时给出明确安装命令，不静默使用最新版。执行基本打包命令：

```powershell
dotnet tool install vpk --version 1.2.161 --tool-path .cache/tools/vpk
& $VpkPath pack --packId YoYoVideo --packVersion $Version --packDir $PackRoot --mainExe $MainExecutable --channel "stable-$Platform" --outputDir $OutputDir
```

不同平台参数从 vpk 1.2.161 help/源码验证后按目标追加，不能用错误参数吞掉退出码；不下载前一版本、不提供 delta 基包。打包前只在验证过的临时 packroot 复制 staging，绝不移动原始 staging 或用户安装目录。
- [ ] **6.3 Windows：** staging 的 bin 内容成为版本目录主程序/运行库，docs/LICENSES/发行说明一起带入；用 Velopack 默认稳定 stub 建立快捷方式。验证 PE 无 console 子系统、正确图标及相邻 DLL，不执行真实用户安装。
- [ ] **6.4 macOS：** 提供 .icns，构造并检查 .app，调整 dylib @loader_path/@rpath 后，vpk 使用 --signAppIdentity - 且不传 notaryProfile。对主程序、更新助手、dylib 与最终 bundle 执行 codesign --verify；otool -L 不得留下 Cellar 或 runner 临时目录依赖。ARM/Intel 分别原生构建。
- [ ] **6.5 Linux：** 独立脚本以 Ubuntu 22.04 runtime 为基线用 ldd/依赖解析组成 AppDir，复制可分发依赖与许可证；排除 glibc 和硬件驱动宿主组件，验证没有缺失依赖。必须包含 libmpv，不复用 .deb 的“依赖系统安装”假设。生成 PNG/.desktop，再由 vpk 生成 AppImage。
- [ ] **6.6 发布签名与验证：** 对 vpk 产出的原生 feed 调用 Task 2 sign/verify；验证只有本平台 full 包、版本一致和实际资源哈希一致。外部工具退出码非零立即失败，不生成假成功标志。
- [ ] **6.7 验证提交：**

```powershell
pwsh -NoProfile -File scripts/test-velopack-package.ps1
git add -- scripts/package-velopack.ps1 scripts/verify-velopack-package.ps1 scripts/test-velopack-package.ps1 scripts/stage-appimage-runtime.ps1 scripts/package.ps1
git commit -m "feat: package signed Velopack releases for desktop targets"
```

## Task 7: 发布工作流与用户文档

**Files:** .github/workflows/release.yml、ci.yml、updater-smoke.yml、scripts/test-updater-release.mjs、scripts/verify-updater-release.mjs、README.md、README-EN.md、docs/development/updater.md。

- [ ] **7.1 RED：** Node test runner 验证发布产物集合构造和 release 校验函数的实际行为，输入缺平台/缺 .sig/版本不一致必须拒绝；不把“YAML 包含某行”当功能测试。独立的 GitHub draft 发布边界由 fake gh 记录命令和退出码，校验只有全部验证成功才进入 publish。
- [ ] **7.2 GREEN：** 复用原 tag/version/notes 流程，增加 setup-dotnet@v4 的 8.0.x 和本地 vpk 1.2.161；Linux 包构建 runner 选择 ubuntu-22.04。将正式安装器步骤从 NSIS 改为 Task 6，不删除旧脚本或覆盖工作区的旧改动。设置：

```yaml
env:
  YOYOVIDEO_UPDATER_PRIVATE_KEY: ${{ secrets.YOYOVIDEO_UPDATER_PRIVATE_KEY }}
  YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD: ${{ secrets.YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD }}
  YOYOVIDEO_UPDATER_PUBLIC_KEY: ${{ vars.YOYOVIDEO_UPDATER_PUBLIC_KEY }}
```

仅受信任 release job 注入私钥。编译前比较 Variable 与固定 updater.pub；PR job 不读生产密钥。四个平台构建/测试完成后在 publish job 合并资产，完整校验→上传 draft→再次下载校验→publish。已有公开同 tag Release 先拒绝，重新发布 0.0.1 时再以明确操作处理，不自动 clobber。
- [ ] **7.3 文档：** 中英文说明各平台的自动更新入口、macOS 未公证限制、.deb/zip 手动升级、密钥备份和 GitHub 变量名；私钥值不出现。记录网络/验签/安装错误日志路径和恢复方法，不建议关闭验签。
- [ ] **7.4 验证提交：**

```powershell
node --test scripts/test-updater-release.mjs
cargo test --workspace
git diff --check
git add -- .github/workflows/release.yml .github/workflows/ci.yml .github/workflows/updater-smoke.yml scripts/test-updater-release.mjs scripts/verify-updater-release.mjs README.md README-EN.md docs/development/updater.md
git commit -m "ci: verify and publish authenticated desktop updates"
```

## Task 8: 实际播放器的版本升级测试

**Files:** scripts/test-velopack-upgrade.ps1、desktop/src/update_qa.rs、desktop Cargo.toml 的专用测试 feature、tests/fixtures/updater/。
**Produces:** 仅测试编译接受 fixture endpoint/public key/user-data-dir；正式编译不接受相关环境变量或 CLI 开关。

- [ ] **8.1 RED：** 实际构建两份独立 fixture 源码/产物版本 0.0.1、0.0.2，使用公开测试密钥和临时用户目录。fixture 的任务消息分别请求 check/download/later/install 并把实际应用版本和播放状态写入测试事件文件。先运行拒绝坏签名/坏包、禁止启动自动应用案例，证明没有验证时会失败。
- [ ] **8.2 GREEN：** 测试专用 feature 实现控制输入但复用 Task 3/5 的正式校验和生命周期；仅替换源、固定测试公钥与目录。脚本启动真实播放器、解码样例媒体、执行下载及稍后、再确认更新；独立助手更新后重启应用，由新进程报告 0.0.2 和历史/偏好保留。脚本设置总超时，失败留证据且清理仅限已核对的测试绝对目录。不得安装到真实用户配置/应用目录。
- [ ] **8.3 平台执行：**

```powershell
pwsh -NoProfile -File scripts/test-velopack-upgrade.ps1 -Platform windows-x64 -FromVersion 0.0.1 -ToVersion 0.0.2
```

相同脚本的三个其他 Platform 分支在各自原生 CI runner 执行；Linux 22.04 与 24.04 的干净测试容器不预装 libmpv，AppImage 使用 FUSE 与 extract-and-run 两种路径。macOS 分别验证 ARM64/Intel，单独记录未公证的正常系统提示。检查同版不提示、占用时不静默杀其他实例、签名/包/缓存篡改不安装。
- [ ] **8.4 正式包检查：** 对非测试 feature 重新构建，证明测试源/公钥/目录开关不可用。测试 0.0.2 包不得上传 stable Release。
- [ ] **8.5 验证提交：**

```powershell
cargo fmt --check
cargo test --workspace
cargo check -p yoyovideo-desktop --features mpv-runtime
git diff --check
git add -- scripts/test-velopack-upgrade.ps1 apps/yoyovideo-desktop/src/update_qa.rs apps/yoyovideo-desktop/Cargo.toml tests/fixtures/updater
git commit -m "test: exercise native player upgrades and preserved state"
```

## Task 9: 最终审查和交付状态

- [ ] 核对每个 spec 章节：1/2/4→Task 1–3；3→全体边界；5/6→Task 3–5；7→Task 6/8；8→Task 2/7；9→Task 1–8；10→范围检查。
- [ ] 检查公钥与已配置 GitHub Variable 一致，不读取/打印远端 Secrets 值；检查 git diff 不含 private key 或密码。
- [ ] 复跑所有本机可运行测试，记录精确命令、通过/忽略/失败数量及平台；未运行的 macOS/Linux 原生测试必须标记未验证。
- [ ] 核对其他工作区改动仍保留；只提交本功能文件。没有用户发布授权时不 push、不改 tag、不覆盖 0.0.1。
- [ ] 报告“本机验证完成”“四目标 CI 完成”“公开 0.0.1 已重发”三个独立状态。不能把前者代替后两者。

## 执行环境注意

本机 rustc 1.94.1，已能访问 crates.io，尚无 dotnet 命令。原生 Windows 打包验证前可在工作区 .cache 下安装 .NET 8 工具链，不修改用户系统环境；随后固定安装 vpk 1.2.161。GitHub CLI 需给工具进程补 APPDATA=C:/Users/Admin/AppData/Roaming，使用既有 keyring 登录；不得再次要求用户上传 token。

若使用并行代理，主代理负责 Task 1–3 和 Cargo manifests；UI 代理只写 Task 4 文件；打包代理只写 Task 6 文件。主代理继续本地阻塞链工作，不重复代理实现。Task 5/7/8 整合后顺序验证。

## 回归测试起点与自检补充

这些测试与每个任务的完整负例清单共同执行，不取代原清单。

### Task 1：篡改清单不得通过验证

~~~rust
#[test]
fn rejects_tampered_manifest_before_parsing() {
    let raw = br#"{"schema_version":1}"#;
    let (public, signature) = signed(raw);
    let altered = br#"{"schema_version":2}"#;
    assert!(VerifiedManifest::verify(altered, &signature, &public, Platform::WindowsX64).is_err());
}
~~~

还需给完整的已签名 abc fixture 写成功断言，保证不能靠总是拒绝通过测试。

### Task 2：错误参数不得变成成功发布

~~~rust
#[test]
fn signer_requires_explicit_inputs() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_yoyo-update-sign"))
        .arg("sign").output().unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
~~~

库接口固定为 sign_release(feed_path: &Path, assets_dir: &Path, platform: Platform, version: &str, public_key: &str, private_key: &str, password: &str) -> Result<(Vec<u8>, String), SignError>；verify_release(raw: &[u8], signature: &str, public_key: &str, platform: Platform, assets_dir: &Path) -> Result<(), SignError>。正例通过真实 sign_release→verify_release 链验证。

### Task 3：手动检查绕过自动节流

~~~rust
#[test]
fn manual_check_bypasses_automatic_interval() {
    let p = UpdatePreferences { automatic_check: true, last_checked_at: Some(1000) };
    assert!(!p.should_check(1001, false));
    assert!(p.should_check(1001, true));
    assert!(p.should_check(87400, false));
}
~~~

### Task 4：真实按钮按阶段门控

复用现有 headless backend 的安装、show、render 和 WindowEvent 指针事件；handler 仅记录 UI 事件，不调用 invoke_install_requested 绕过按钮。

~~~rust
let installs = std::rc::Rc::new(std::cell::Cell::new(0));
window.on_install_requested({
    let installs = installs.clone();
    move || installs.set(installs.get() + 1)
});
window.set_phase_index(6);
~~~

在 ReadyToInstall 点击安装次数为 1；切换 Downloading 后同位置点击不能增加次数。zh/en 都运行，按钮坐标从测试布局定义核对，禁止为测试给生产 UI 开后门。

### Task 5/8：SDK 不得偷偷安装缓存

真实安装 fixture 启动前放入已下载但未确认的新版包；启动后主窗口存在且版本仍为 0.0.1，点击“稍后”再启动仍为 0.0.1。普通单测验证状态机和调用顺序，原生 suite 验证真正进程行为。

### Task 6：缺运行库的 staging 不可发布

~~~powershell
& pwsh -NoProfile -File scripts/package-velopack.ps1 -Platform windows-x64 -PackageDir $EmptyFixture -Version 0.0.1 -OutputDir $FixtureOutput
if ($LASTEXITCODE -eq 0) { throw "Missing runtime accepted" }
if (Test-Path -LiteralPath (Join-Path $FixtureOutput "yoyovideo-update.windows-x64.json")) { throw "Invalid package was marked publishable" }
~~~

EmptyFixture 和 FixtureOutput 在测试前由 GetTempPath 与 GUID 创建；清理前验证绝对路径始终位于测试根目录内。

### Task 7：平台集合必须完整

新增 scripts/verify-updater-release.mjs，导出 validatePlatformSet(platforms: string[])；生产 CLI 读取签名清单后调用相同函数。代码必须验证四个精确且不重复的 target，返回 void 或抛错。

~~~javascript
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { validatePlatformSet } from './verify-updater-release.mjs';
test('refuses a partial release', () => {
  assert.throws(() => validatePlatformSet(['windows-x64']));
  assert.doesNotThrow(() => validatePlatformSet([
    'windows-x64', 'macos-aarch64', 'macos-x86_64', 'linux-x64',
  ]));
});
~~~

### Task 8：必须来自新版真实进程

fixture events.jsonl 记录 PID、真实编译版本、历史位置；区分旧进程、更新助手和重启进程。只有新进程报告 0.0.2，且保存的媒体路径与非零播放历史存在，才通过。文件名变化、助手返回 0 都不足以证明升级成功。重启不自动播放，由测试专用控制命令重新打开媒体验证恢复。

## 计划自检结果

- 协议、安全、UI、退出、打包、CI、原生测试均已映射 Task 1–9。
- Task 4 仅写 UI，不与 Task 5 Rust 接线冲突；Task 6 不改 Cargo manifests。
- 接口生产者与消费者名称一致，测试用真实验签、文件和状态决策。
- Windows 本机缺 dotnet 属于工具准备；macOS/Linux 原生验证必须在对应环境执行，未执行不得标为通过。
