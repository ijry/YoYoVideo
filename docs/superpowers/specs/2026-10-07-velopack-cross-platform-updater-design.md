# YoYoVideo 三平台 Velopack 自动升级设计

状态：设计待审阅；本文不是实现或发布完成声明。

## 1. 目标与已确认约束

- 为 Windows、macOS、Linux 提供统一的检查、下载、验签、安装和重启体验，不更换 Slint UI 框架。
- 使用 Velopack；以重新发布的 0.0.1 为升级起点，不迁移既有 NSIS 安装、历史发行包或旧版本更新协议。
- 保持 Cargo.toml 的 workspace.package.version 为唯一应用版本来源；真正的后续更新必须高于当前版本。
- 首期使用全量更新包，不生成或应用差分更新。全量包必须包含同版本应用和所需 libmpv 运行库。
- 支持 windows-x64、macos-aarch64、macos-x86_64、linux-x64；这里的“三平台”指桌面系统，不包含移动端或尚无构建任务的 CPU 架构。
- Windows 安装器改用 Velopack；macOS 交付 .app；Linux 自更新交付 AppImage。保留 .deb 作为系统包管理渠道，不让应用自行覆盖 .deb 安装。
- macOS 使用 ad-hoc 签名，不做 Apple 公证，不宣称能够绕过 Gatekeeper。Windows 暂无受信任 Authenticode 证书，不承诺消除 SmartScreen 提示。
- 现有用户配置、播放历史和字幕偏好继续存放在 AppPaths 指定的用户目录，不放进可被更新替换的应用目录。
- 本任务不擅自移动/强推 v0.0.1 tag 或立即覆盖公开 Release。完成实现、验证后，单独执行已确认的重新发布步骤。

## 2. 现状与不能忽略的行为

当前 main.rs 直接进入播放器；app.rs 在 Slint 事件循环退出后保存播放历史、字幕偏好和标记，视频渲染对象与 mpv 的释放顺序也有要求。

现有发布流水线覆盖四个系统/架构目标，具有版本/tag 一致性检查、libmpv staging、许可文件检查和运行时冒烟测试。这些约束继续保留，不为引入更新器而跳过播放验证。

已核对 Velopack 1.2.161 源码：

- VelopackApp 默认启用启动时安装缓存更新；本项目必须显式关闭，避免未经本应用再次核验和用户确认就安装。
- apply_updates_and_restart 会调用 process::exit(0)；播放器不能在普通按钮回调中直接调用它，否则可能跳过状态保存和 Rust 对象析构。
- wait_exit_then_apply_updates 会启动更新助手并等待当前进程正常退出，适合本项目；助手默认等待上限为 60 秒。
- SDK 支持 SHA-256 校验，但输入未提供 SHA-256 时会退回 SHA-1。本项目自己的协议必须要求 SHA-256，拒绝缺失或无效值。
- SDK 的普通 GitHub/HTTP 更新源不消费 Tauri 的 minisign 配置，需要应用实现带验签的 UpdateSource，不能只上传密钥就宣称更新可信。

工作区已有的安装器关闭进程相关改动属于其他工作，实施时不得重置或顺手提交。新增 Velopack 脚本独立放置；旧安装器脚本不再进入新的正式发布主路径，但不删除用户未提交内容。

## 3. 依赖和模块边界

固定 Rust velopack crate 与 vpk CLI 为 1.2.161，二者同步升级。CI 使用 .NET 8 SDK 运行这一版 vpk；终端用户不因此需要安装 .NET。其余 Rust 依赖使用 Cargo.lock 固定，保留本仓库现有 Rust 版本声明并验证依赖兼容性，不静默抬高版本要求。

组件划分：

1. crates/yoyo-updater：签名清单、平台/版本策略、更新状态、网络来源和 Velopack 适配，不依赖 Slint 或 libmpv。
2. crates/yoyo-update-sign：小型发布工具，读取已经生成的 Tauri/minisign 格式密钥，生成及校验更新清单签名；与播放器解耦，不引入 Tauri 运行时。
3. apps/yoyovideo-desktop/src/update_runtime.rs：后台任务、Slint 消息投递和更新窗口生命周期。
4. apps/yoyovideo-desktop/src/app.rs / main.rs：最小化接线，负责启动入口、正常退出和现有状态保存；不把下载实现继续堆进 app.rs。
5. apps/yoyovideo-desktop/ui/update-window.slint：独立更新窗口，避免原生视频子窗口遮挡弹出层。
6. scripts/package-velopack.ps1 与验证脚本：消费现有 runtime staging，生成、签名和检查各平台更新产物。
7. .github/workflows/release.yml：协调构建、签名、验证和发布，不在工作流 YAML 内复制第二套升级协议实现。

正式应用内置更新公钥。新增公共资源 apps/yoyovideo-desktop/assets/updater.pub，内容必须与已经上传的 Repository Variable 相同。CI 比较两者，不允许仅修改 Variable 就悄悄更换客户端信任根。

## 4. 更新源与认证协议

公开更新源为 ijry/YoYoVideo 的 GitHub Releases，不给客户端配置 GitHub token，也不需要额外升级服务器。

每个平台有独立 Velopack channel：stable-windows-x64、stable-macos-aarch64、stable-macos-x86_64、stable-linux-x64。首期仅开放 stable，忽略预发布版本，不提供渠道切换 UI。

每个 Release 上传以下协议文件：

- yoyovideo-update.<platform>.json：带上下文的更新清单。
- yoyovideo-update.<platform>.json.sig：对上述原始字节的 detached minisign 签名，采用现有密钥兼容的编码。
- vpk 生成的原生 feed 和全量 .nupkg；客户端不直接信任未签名的原生 feed。

签名清单的顶层字段固定为：

| 字段 | 校验规则 |
| --- | --- |
| schema_version | 必须为 1；未知版本拒绝 |
| app_id | 必须为 YoYoVideo |
| platform | 必须与当前构建目标完全一致 |
| channel | 必须与当前平台的 stable channel 完全一致 |
| version | 合法 SemVer，匹配全量包版本 |
| release_tag | 必须为 v 加 version，下载地址由此构造 |
| feed | VelopackAssetFeed；仅包含当前版本的完整包，不接受 delta 或其他版本混入 |

feed 内必须含目标 PackageId、Version、FileName、Size、非空有效 SHA256。保留 SDK 要求的 SHA1 字段，但安全判定不能回退到 SHA1。包文件名只能是单个文件名，不允许绝对路径、目录分隔符、冒号、父目录引用或重复条目。

验证顺序：

1. 从固定仓库的 latest/download 路径下载清单及签名；只接受 HTTPS，并限制重定向、超时和响应体大小。
2. 用内置公钥校验清单原始字节及 minisign 可信注释签名；失败就停止，不回退到未签名 feed。
3. 校验清单上下文、版本、平台、channel、文件名、大小和哈希格式。更新说明以纯文本显示，不执行 HTML。
4. 由签名中的 release_tag 和经过验证的包文件名构造固定仓库的下载 URL，不执行来自清单的命令。
5. 冻结本次检查得到的清单和签名，后续下载只使用这份已验证快照，不能在下载中途悄悄换成新的 latest。
6. 下载后和实际安装前均校验文件大小、SHA-256，包括 SDK 复用已有缓存的情况；未通过校验的文件不得进入 ReadyToInstall。

元数据上限 8 MiB，包大小上限 2 GiB，连接超时 10 秒，元数据请求总超时 30 秒，单次包下载总超时 30 分钟。首期不做断点续传；超时/中断后可重新下载。正式构建不接受环境变量关闭验签或改写任意更新源。

本协议防止发布资源被替换、跨平台清单混用和降级；不声称能抵御本机用户权限已完全失陷或签名私钥泄露。客户端始终拒绝低于或等于当前版本的自动安装，旧签名清单被重放最多造成暂时看不到新版，不能触发降级。

## 5. 状态机和用户体验

状态为 Unsupported、Idle、Checking、UpToDate、Available、Downloading、ReadyToInstall、PreparingInstall、Error。

- 菜单增加“检查更新 / Check for Updates”，打开独立窗口。
- 展示当前版本、新版本、纯文本发布说明和下载进度；中文/英文跟随现有语言设置。
- 用户明确点击下载后才下载，不因发现新版就消耗大流量。
- 下载完成后提供“立即重启更新”和“稍后”；关闭窗口不等于同意安装。
- 首次启动后延迟 10 秒进行后台检查，此后自动检查最多每 24 小时一次；提供“自动检查更新”开关，默认开启。
- 更新时间和开关保存到 config_dir/updater.toml；手动检查不受 24 小时节流限制。
- 自动检查失败只记录诊断，不打断播放。手动失败显示可重试的简明原因。
- 同一时间仅允许一个检查/下载任务；按钮禁用和任务编号共同防止重复启动与过期回调覆盖新状态。
- 网络和文件哈希操作在工作线程完成，通过消息传回 UI；不得阻塞 Slint 事件循环。
- 关闭更新窗口时下载可以继续，完成后只显示可见提示，不强制弹出窗口或重启。
- “稍后”保留已验证的候选信息和包。下次启动重新验签、验包后恢复提示，但不得由 SDK 自动安装。

开发运行、旧 zip/tar 分发和 .deb 无 Velopack 安装上下文时进入 Unsupported：仍可正常播放，更新窗口说明当前渠道不支持应用内安装并提供官方发布页面入口，不尝试覆盖任意目录。

## 6. 安装与正常退出

main() 的第一项应用逻辑执行 VelopackApp::build().set_auto_apply_on_startup(false).run()，然后再启动 Slint、读取媒体参数或初始化 mpv；安装钩子不得打开播放器窗口。

用户点击“立即重启更新”后的顺序固定为：

1. 禁止新增更新任务，复验冻结的签名清单及缓存包，任何错误回到可重试状态。
2. 在 UI 所在线程抓取当前播放/字幕/标记状态，保存历史、偏好和窗口位置。保存失败应提示并保留播放器，不继续安装。
3. 调用 wait_exit_then_apply_updates，设置等待当前进程、显示安装进度并在成功后重启。启动助手失败时保持窗口和播放器存活。
4. 助手启动成功后退出所有应用窗口和事件循环，执行现有收尾；停止后台任务，按原有依赖顺序释放渲染上下文和 mpv。
5. 正常返回进程入口，让独立助手在进程结束后替换完整版本目录并启动新版。

不得在普通按钮回调中调用会立即 process::exit 的快捷安装 API，不得使用 NoWait；播放器本身不执行强制杀进程操作。

Windows 的 Velopack 助手可能结束同一版本目录中的占用进程。因此应用层在安排安装前检查同一安装目录的其他播放器实例和可检测的文件占用；存在其他实例时提示用户先关闭。安装确认明确提示更新期间不要启动其他实例，不能把这一检查描述为消除了全部占用竞态。更新助手的失败/日志必须可定位；不得把“进程已经退出”当成“新版安装成功”。

重启默认不自动播放视频；历史和播放位置仍然保留，用户再次打开时按既有恢复策略处理。不要把来源不明的更新清单字段变成重启参数。

## 7. 各平台包与依赖

### Windows x64

新正式安装器使用 Velopack 的版本目录、启动 stub 和 Update.exe；不继续由 NSIS 覆盖安装。携带同一构建的 mpv-2.dll 与实际运行依赖，保留图标、许可和发行说明。快捷方式指向 Velopack 稳定启动入口；vpk 1.2.161 原生安装验收确认实际目标为 `current/yoyovideo-desktop.exe`，不是版本号目录或根目录 stub。首期不承诺普通 zip 具有自更新能力。

### macOS ARM64 / x86_64

分别构建两个 .app，保持当前架构区分，不伪装成 Universal。把 libmpv 及递归 dylib 依赖放入应用包，调整 @loader_path/@rpath，打包后验证不存在 Homebrew Cellar 或本机构建目录的绝对依赖。

使用 vpk 的 --signAppIdentity '-'；不传 --notaryProfile，不产生 Apple Developer ID 的虚假标识。签名覆盖自带 dylib、更新助手和最终 .app；打包前修改依赖路径，签名后不再修改可执行内容。测试产物仍可能需要用户通过 macOS 的正常界面确认运行。

### Linux x64

Velopack 路径交付 AppImage；不只是将现有依赖系统 libmpv 的 .deb 外壳改名。AppDir 必须包含主程序、libmpv 与可再分发的依赖闭包，保留系统图形驱动与 glibc 边界。首次目标运行基线为 Ubuntu 22.04 x86_64，在 22.04 和 24.04 干净环境验证，无预安装 mpv/Homebrew 依赖。

使用独立 AppDir 组装和运行库检查，不能改变 .deb 继续声明系统依赖的语义。AppImage、图标和 .desktop 信息保持一致；以 --appimage-extract-and-run 路径补充无 FUSE 环境的 CI 检查。用户将 AppImage 放在受保护目录时，不自动进行无提示提权。

## 8. 签名密钥与 CI

现有密钥位于 D:/Repos/xyito/config/yoyovideo，公钥标识 DD3DF8EED25DA3E5。私钥与密码不得复制进仓库、产物或日志。该密钥用于更新认证，不是 Apple/Windows 系统信任证书。

目标仓库 ijry/YoYoVideo 已配置：

- Secret YOYOVIDEO_UPDATER_PRIVATE_KEY。
- Secret YOYOVIDEO_UPDATER_PRIVATE_KEY_PASSWORD。
- Variable YOYOVIDEO_UPDATER_PUBLIC_KEY。

发布工具只从标准输入、受保护临时文件或进程环境接收私钥/密码，不放入命令行参数。构建前验证所配置公钥与客户端固定公钥一致；签名后重新执行完整密码学验签，不仅比较 key ID。

签名仅在受信任的 tag/release 工作流执行；普通 PR 测试只使用固定的公开测试密钥，不能申请生产 Secrets。临时密钥文件用平台原生安全文件操作清理，不跨 shell 拼接删除命令。

新的 release 工作流顺序：

1. 核对 annotated tag、Cargo 版本和发布说明，固定同一提交。
2. 四个构建目标分别准备 runtime、编译正式播放器、构造 Velopack 包并运行包/播放验证。
3. 生成签名清单，验证平台、版本、SHA-256 和签名，收集全部目标产物。
4. 所有目标成功后上传到 draft Release；四套包、签名清单和签名齐全才对外发布。
5. 校验发布资源能下载、签名有效、latest 指向预期版本。任一目标失败，不发布半套更新。

保留现有 runtime 许可证、来源和 release notes；Release 下载表区分“支持自动升级”和“.deb/普通压缩包手动升级”。首次替换既有 v0.0.1 Release 的破坏性操作只在最终发布步骤明确处理，不在测试脚本中执行。

## 9. 测试与验收

先写失败测试，再实现对应功能。测试分为可离线运行的契约测试与真实平台安装测试，不能互相替代。

### 离线契约测试

- 正确签名、错误公钥、正文/可信注释篡改、签名缺失或截断。
- 错误 app_id/platform/channel、无效版本、同版/降级、无 SHA-256、超大清单和路径穿越文件名。
- 下载后篡改、缓存命中后篡改、安装前复验失败均不调用安装器。
- 开发/.deb 无安装上下文不崩溃、不修改安装目录。
- 检查节流、重复请求、过期结果、错误重试、下载进度、稍后安装和启动时禁止自动应用。
- 持久化失败或助手启动失败不退出；正确顺序为验签/验包、保存、安排等待退出、正常清理。
- 更新 UI 的中英文、800×600 主窗口入口、独立窗口按钮和状态切换。

### 四个构建目标的端到端测试

在独立临时安装目录、用户数据目录及本地测试更新源中构建 0.0.1 与 0.0.2 包，使用测试密钥；测试更新源只通过测试入口注入，不给正式客户端留下关闭验签的开关。

必须验证：安装 0.0.1、正常播放样例媒体、发现并下载 0.0.2、稍后仍可播放、确认后正常退出、助手完成替换、新进程版本为 0.0.2、配置与历史仍在、同版不再提示。额外覆盖坏签名/坏包拒绝、文件占用失败，以及 macOS ARM64/Intel 与 Linux 无系统 mpv 的真实运行。

CI 上 macOS/Linux 验证失败或尚未执行时，交付说明必须写清“未验证”，不能仅凭 Windows 测试宣称三平台完工。0.0.2 验证包只作为测试产物，不发布到正式 stable Release。

### 完成定义

只有当客户端、验签、四套平台包、GitHub 发布链路和上述测试通过，才可称“自动升级实现完成”；之后单独确认并重新发布 0.0.1。密钥上传、文档完成、cargo check 通过都不能单独算作功能完成。

## 10. 明确不做的内容

旧版迁移、差分更新、静默强制重启、移动平台、额外 CPU 架构、应用内更新 .deb、Apple 公证、购买代码签名证书、自建升级服务器、遥测和无关播放器重构不在本期范围内。

## 11. 查证依据

- https://docs.velopack.io/getting-started/rust
- https://docs.velopack.io/packaging/operating-systems/windows
- https://docs.velopack.io/packaging/operating-systems/macos
- https://docs.velopack.io/packaging/operating-systems/linux
- https://github.com/velopack/velopack/blob/1.2.161/src/lib-rust/src/app.rs
- https://github.com/velopack/velopack/blob/1.2.161/src/lib-rust/src/manager.rs
- https://github.com/velopack/velopack/blob/1.2.161/src/lib-rust/src/sources/mod.rs
- ai-switch 本地 src-tauri/tauri.conf.json：macOS signingIdentity 为 '-'；release.yml 使用更新签名 Secrets。
