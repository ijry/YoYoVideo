# 一键隐私模式 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. User preference: inline/sequential execution, no subagents. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 交付受保护媒体名单、4 位 PIN、周期日程与手动覆盖，以及单视频/多宫格的真实暂停、静音和遮挡。

**Architecture:** core 提供媒体标识、纯日程策略和播放准入接口；桌面独立模块负责 PIN、原子配置、后台验证和 Slint 窗口。播放安全在 AppSession/后端边界执行，画面安全由原生窗口与合成纹理的可见性许可执行，而不是只隐藏按钮。

**Tech Stack:** Rust、Slint 1.17、chrono 0.4、Argon2id、serde/toml、libmpv、现有 Windows/macOS/X11/Wayland 视频宿主。

**Spec:** `docs/superpowers/specs/2026-10-08-privacy-mode-design.md`（用户已审阅确认）

## Global Constraints

- 名称“一键隐私模式”；4 个 ASCII 数字 PIN，支持前导零，不明文保存，不使用更新签名密钥。
- 配置后开启免 PIN；手动关闭、保护名单/日程/PIN 修改需要 PIN；5 次失败冷却 30 秒，冷却持久化。
- 手动覆盖到下一段合并后受限周期的开始才失效，不在当前周期结束时失效；覆盖与截止点跨重启保存。
- 受限区间 `[start, end)`；星期归属于开始当天；跨午夜、相接/重叠合并；相等起止时间拒绝。
- 已打开/加载/播放/暂停的受保护条目都必须隐藏；先禁止输出并静音，再幂等暂停，失败保持遮挡并卸载。
- 普通网格条目继续播放；解除不自动播放，不清除原历史、进度、字幕偏好或标记。
- 日程用本地时区，测试注入时钟；不修改系统时间。隐私占位必须不透明。
- 只控制当前进程的媒体会话，不提供系统级加密、防截图或跨进程即时广播。
- 既有旧安装器相关 8 个未提交文件必须保留；只提交明确归属路径，不 `git add -A`。
- 留在 `feat/privacy-mode`；不改 main/发布 tag，不推送或重新发布 0.0.1。
- Windows Cargo 验证 `-j 2`；运行 mpv 测试时 PATH 加 `third_party/mpv/windows-x64/bin`。

## 文件边界

- `crates/yoyo-core/src/privacy/{mod.rs,identity.rs,schedule.rs}`：媒体标识、日程/覆盖数据和纯计算。
- `crates/yoyo-core/src/{backend.rs,app_command.rs,session.rs}`：共享准入接口、幂等暂停、事务式打开、保护静音和事件防守。
- `apps/yoyovideo-desktop/src/privacy/{mod.rs,credentials.rs,store.rs,service.rs,window.rs}`：认证、持久化、后台任务、窗口绑定。
- `apps/yoyovideo-desktop/ui/privacy-window.slint`：设置/验证 PIN、日程编辑、名单管理。
- `apps/yoyovideo-desktop/src/{app.rs,grid_runtime.rs,video_host.rs,video_host_winit.rs,video_surface_gl.rs}`：必要接线和每个实际输出表面的隐藏。
- `crates/yoyo-core/src/shortcut.rs` 与桌面语言/设置映射：新增无默认绑定的隐私快捷键动作。
- 对应 core/desktop 单元和集成测试；原生验收沿用隔离测试目录，不读取用户影片。

## Task 1: 媒体标识与日程策略

**Files:** 新建 core/privacy 三个模块、`crates/yoyo-core/tests/privacy_policy_contract.rs`；修改 core Cargo.toml、lib.rs、Cargo.lock。

**Interfaces:**
```rust
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MediaKey(String);
impl MediaKey {
    pub fn from_locator(locator: &MediaLocator) -> Result<Self, String>;
    pub fn as_str(&self) -> &str;
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacyTimeRule { pub weekdays: u8, pub start_minute: u16, pub end_minute: u16 }
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivacySchedule { pub enabled: bool, pub rules: Vec<PrivacyTimeRule> }
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManualPrivacy { pub enabled: bool, pub until: Option<DateTime<Utc>> }
pub struct PrivacyDecision { pub enabled: bool, pub manual: bool, pub next_start: Option<DateTime<Utc>> }
impl PrivacySchedule {
    pub fn validate(&self) -> Result<(), String>;
    pub fn evaluate<Tz: TimeZone>(&self, now: DateTime<Utc>, tz: &Tz) -> PrivacyDecision;
    pub fn with_override<Tz: TimeZone>(&self, value: Option<&ManualPrivacy>, now: DateTime<Utc>, tz: &Tz) -> PrivacyDecision;
}
```

- [x] 1.1 RED：加入可编译接口和以下行为用例，先运行并确认是策略缺失失败，不把语法错误当 RED。
```rust
let schedule = PrivacySchedule { enabled: true, rules: vec![PrivacyTimeRule {
    weekdays: 0b1111111, start_minute: 9 * 60, end_minute: 18 * 60,
}] };
let now = Utc.with_ymd_and_hms(2026, 10, 8, 10, 0, 0).unwrap();
let next = Utc.with_ymd_and_hms(2026, 10, 9, 9, 0, 0).unwrap();
assert!(schedule.evaluate(now, &Utc).enabled);
let manual = ManualPrivacy { enabled: false, until: Some(next) };
assert!(!schedule.with_override(Some(&manual), now, &Utc).enabled);
assert!(schedule.with_override(Some(&manual), next, &Utc).enabled);
```
- [x] 1.2 实现身份规范化：文件 canonicalize 成功时使用真实绝对路径，否则使用词法归一化绝对路径；Windows 统一分隔符/大小写，保留既有历史键。URL 用 url crate 规范化，不丢查询参数。
- [x] 1.3 展开本地日期前 8 天至后 15 天的规则，转换起止为 UTC 后排序、合并相接区间；当前覆盖求 active，未来起点求 next_start。DST 开始取 earliest、结束取 latest；不存在的分钟向后找首个有效分钟。
```rust
intervals.sort_by_key(|i| i.start);
for interval in intervals {
    if let Some(last) = merged.last_mut().filter(|last| interval.start <= last.end) {
        last.end = last.end.max(interval.end);
    } else { merged.push(interval); }
}
let manual = manual.filter(|m| m.until.is_none_or(|end| now < end));
```
- [x] 1.4 GREEN：`cargo test -p yoyo-core --test privacy_policy_contract -j 2`；补星期/跨夜/相接合并、无日程无限覆盖、手动开启跨当前结束、过期覆盖、URL 和文件别名案例。
- [x] 1.5 仅提交本任务文件：`feat: model privacy schedules and manual overrides`。

## Task 2: PIN 与原子配置

**Files:** 新建 desktop/privacy/{mod.rs,credentials.rs,store.rs} 及模块单元测试；修改 desktop Cargo.toml、lib.rs、Cargo.lock。

**Interfaces:**
```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct PinCredential { encoded: String }
// PrivacyError is a redacted error enum in privacy/mod.rs: InvalidPin,
// ConfirmationMismatch, Unconfigured, Locked, Busy, Stale, Cooldown(u32),
// CorruptConfig, Persistence. Display/Debug never includes PINs or PHC data.
impl PinCredential {
    pub fn create(pin: &str) -> Result<Self, PrivacyError>;
    pub fn verify(&self, pin: &str) -> Result<bool, PrivacyError>;
}
#[derive(Clone, Serialize, Deserialize)]
pub struct PrivacyDocument {
    pub schema_version: u32, pub revision: u64,
    pub credential: Option<PinCredential>,
    pub failed_attempts: u8, pub blocked_until: Option<DateTime<Utc>>,
    pub schedule: PrivacySchedule, pub manual: Option<ManualPrivacy>,
    pub protected: BTreeMap<MediaKey, MediaLocator>,
}
pub struct PrivacyStore { path: PathBuf }
impl PrivacyStore {
    pub fn new(path: PathBuf) -> Self;
    pub fn load(&self) -> Result<PrivacyDocument, PrivacyError>;
    pub fn save(&self, value: &PrivacyDocument) -> Result<(), PrivacyError>;
}
```

- [x] 2.1 RED：真实 Argon2/临时目录用例，不 mock 哈希。
```rust
let a = PinCredential::create("0123").unwrap();
let b = PinCredential::create("0123").unwrap();
assert!(a.verify("0123").unwrap());
assert!(!a.verify("1230").unwrap());
assert_ne!(toml::to_string(&a).unwrap(), toml::to_string(&b).unwrap());
assert!(PinCredential::create("123").is_err());
assert!(PinCredential::create("１２３４").is_err());
```
- [x] 2.2 使用 Argon2id v19（19 MiB、t=2、p=1、随机 16 字节 salt、32 字节输出）；校验读取的 PHC 类型与参数上限，防止恶意配置触发无界内存。Debug 不输出校验值，PIN 临时字符串用 zeroize 清理。
- [x] 2.3 文档 schema=1、最多 64 规则/10000 媒体/1 MiB；无 PIN 的配置不能带名单/日程/覆盖。缺文件为未配置；已有坏文件报错，不返回无保护默认值。
- [x] 2.4 写临时文件、flush/sync、rename 替换；Unix 文件模式 0600。失败保持原文件；不用递归删除用户目录。保存只影响 privacy.toml。
```rust
let before = std::fs::read(&path).unwrap();
assert!(store.save(&invalid_document).is_err());
assert_eq!(std::fs::read(&path).unwrap(), before);
```
- [x] 2.5 GREEN：`cargo test -p yoyovideo-desktop --lib privacy:: -j 2`；检查序列化无 PIN、坏 schema/hash/文件拒绝、前导零和失败保存。
- [x] 2.6 仅提交该模块和依赖：`feat: persist PIN-protected privacy preferences`。

## Task 3: 不可绕过的播放准入和幂等保护

**Files:** core/backend.rs、app_command.rs、session.rs、session/privacy.rs、lib.rs；新增 `tests/privacy_playback_contract.rs`。

**Interfaces:**
```rust
pub trait PlaybackAccess: Send + Sync { fn restricted(&self, media: &MediaKey) -> bool; }
impl<B: PlayerBackend> AppSession<B> {
    pub fn set_playback_access(&mut self, access: Arc<dyn PlaybackAccess>);
    pub fn privacy_blocked(&self) -> bool;
    pub fn enforce_privacy(&mut self) -> Result<(), AppError>;
    pub fn current_media_key(&self) -> Option<&MediaKey>;
}
// AppCommand::SetPaused(bool)；默认 AppSession 无策略，旧调用行为不变。
```

- [x] 3.1 RED：可切换的真实策略 + 记录后端边界；从打开、播放中切换、下一集/EOF、截图四类入口断言。
```rust
session.set_playback_access(guard.clone());
session.handle_command(AppCommand::OpenFile(file.clone())).unwrap();
guard.block.store(true, Ordering::SeqCst);
session.enforce_privacy().unwrap();
assert!(session.state().paused);
assert!(session.backend().commands.contains(&BackendCommand::SetPaused(true)));
let count = session.backend().opens.len();
assert!(session.handle_command(AppCommand::TogglePause).is_err());
assert!(session.handle_command(AppCommand::TakeScreenshot(output)).is_err());
assert_eq!(session.backend().opens.len(), count);
```
- [x] 3.2 所有三条 backend.open 路径先检查、后打开成功才提交队列/当前媒体/轨道状态；拒绝时不先修改选择和 current。EOF 连播/循环复用相同入口。
- [x] 3.3 增加 SetPaused；恢复、seek/step/screenshot 等输出命令在受限时拒绝。保护静音单独保存，有效值 `user_muted || privacy_muted`，不能把保护反馈写成用户静音偏好。
- [x] 3.4 受限时无论原来播放或暂停，都静音并下发 SetPaused(true)；重复检查幂等。若暂停失败，Stop 后端但保留恢复定位/进度，不自动重放；异常恢复事件重新阻止输出。
- [x] 3.5 GREEN：`cargo test -p yoyo-core --test privacy_playback_contract -j 2`，再跑 core 全部测试；补暂停失败、拒绝不改变队列、解除仍暂停、音量/静音偏好保留。
- [x] 3.6 提交：`feat: enforce privacy at playback boundaries`。

## Task 4: 桌面策略服务与认证生命周期

**Files:** desktop/privacy/service.rs 与模块内单元测试；privacy/mod.rs。

**Interfaces:** 公开 API 不允许外部构造“已验证”结果，避免在正式构建留下免密入口。

- PrivacyClock: Send + Sync 提供 fn now(&self) -> DateTime<Utc>；生产 SystemPrivacyClock，测试模块内实现可变时钟。
- PrivacyService::load(store: PrivacyStore) -> Self（可注入时钟的 with_clock 仅在 test/privacy-qa 中编译），服务可 Clone，内部共享状态；读取失败记为 fail-closed。
- snapshot() -> PrivacySnapshot：configured/enabled/manual/next_start/blocked_until/fail_closed/dirty，不含凭据、PIN 或受保护标签。
- enable() -> Result<(), PrivacyError>：立即发布内存保护；flush() -> Result<(), PrivacyError> 原子写回。
- AuthPurpose::{Setup, Unlock, Settings, ChangePin}；begin_auth(purpose) -> Result<AuthTicket, PrivacyError> 生成不可伪造的私有字段 ticket，含 request、revision 和 next_cycle。
- setup(ticket, pin: &str, confirmation: &str) -> Result<AuthGrant, PrivacyError> 与 authenticate(ticket, pin: &str) -> Result<Option<AuthGrant>, PrivacyError> 在后台调用；Unlock 返回 None，Settings 返回仅本窗口可用的 grant。成功放行前必须重新验证 ticket 并写盘。哈希期间不持有状态锁。
- cancel_authorization() 撤销 ticket/grant；tick() 发现跨新周期时撤销；准入本身用当前时间计算，不能等 tick。
- settings(&AuthGrant) -> Result<PrivacySettings, PrivacyError> 返回 schedule 和 protected 列表；set_schedule(&AuthGrant, PrivacySchedule)、set_protection(&AuthGrant, MediaLocator, bool)、change_pin(&AuthGrant, &str, &str) 均返回 Result<(), PrivacyError>，在后台串行执行。更改 PIN 需重新验证旧 PIN 后取得新 grant。
- impl PlaybackAccess for PrivacyService；所有 mutation 检查 epoch/周期截止点，降低保护只有保存成功后发布；开启保存失败保持 dirty+受限。

- [x] 4.1 RED：模块内临时目录 + 测试时钟 + 真实凭据；以下测试先证明未设置不能免密开启、配置后可以开启。

test body (imports: std::sync::Arc and super::*):

    let dir = tempfile::tempdir().unwrap();
    let service = PrivacyService::load(PrivacyStore::new(dir.path().join("privacy.toml")));
    assert!(service.enable().is_err());
    let ticket = service.begin_auth(AuthPurpose::Setup).unwrap();
    service.setup(ticket, "0123", "0123").unwrap();
    service.enable().unwrap();
    assert!(service.snapshot().enabled);
    let ticket = service.begin_auth(AuthPurpose::Unlock).unwrap();
    service.cancel_authorization();
    assert!(service.authenticate(ticket, "0123").is_err());
    assert!(service.snapshot().enabled);

- [x] 4.2 为错误五次/冷却重启、新周期旧 reply、取消、再开启、失焦、落盘失败分别写失败测试；测试 clock 仅定义于 cfg(test)，使用 Local 构造手算开始点，不改系统时间。
- [x] 4.3 实现 ticket/grant 生命周期及后台调用接口：开始请求时拒绝 cooldown/busy；失败验证即使已取消也保守计数；成功结果只有 revision/request/epoch/截止点全匹配才能应用。
- [x] 4.4 用版本化共享状态防止旧写入覆盖新保护；串行存储，发布降低前复验；flush 保存最新内存版本；损坏配置全媒体受限且不覆盖原文件。
- [x] 4.5 tick 清理过期 manual；set_schedule 重算仍有效 manual 的下个开始；PlaybackAccess 每次读取时钟，设置名单与错误信息只通过授权接口返回。
- [x] 4.6 GREEN：cargo test -p yoyovideo-desktop --lib privacy:: -j 2，覆盖 4.1–4.5 所有反例。
- [x] 4.7 提交：feat: coordinate privacy authentication and time transitions。

实现补充：PIN 工作分为后台 prepare_setup/verify_pin/prepare_pin_change 与主线程 apply_setup/apply_verification/apply_pin_change。返回结构字段私有，不能伪造验证结果。主线程先处理窗口失焦/取消，再接收回复；成功的后台校验本身绝不放行视频。磁盘短写入串行并与发布同步，昂贵 Argon2 不持有状态锁。

## Task 5: 真正隐藏视频表面和多宫格保护

**Files:** desktop/video_host.rs、video_host_winit.rs、video_surface_gl.rs、grid_runtime.rs 及对应测试。

**Interfaces:**
```rust
// VideoHost 增加 set_privacy_blocked(bool)，默认实现允许旧测试替身继续工作。
// VisibilityPermit: Clone + Default; shared atomic requested/blocked flags.
// request_visible(&self, bool), set_privacy_blocked(&self, bool), visible(&self) -> bool.
// WinitVideoHost owns this permit; queued work reads latest visible().
// GridTileView 增加 privacy_blocked；每个 tile 的 session 共享 PlaybackAccess。
```

- [x] 5.1 RED：记录宿主证明 popup 解除不清隐私、旧 show 任务不覆盖新 lock、网格仅受保护格子隐藏。
```rust
let visibility = VisibilityPermit::default();
visibility.request_visible(true);
let pending = visibility.clone(); // queued work captures the shared permit
visibility.set_privacy_blocked(true);
assert!(!pending.visible());
visibility.request_visible(false);
visibility.set_privacy_blocked(false);
assert!(!pending.visible());
visibility.request_visible(true);
assert!(pending.visible());
```
- [x] 5.2 独立合并弹窗/隐私原因；实际可见性为请求可见且所有许可通过。macOS 沿用事件循环排队、weak window，不在借用 Runtime 时同步回调 AppKit。
- [x] 5.3 合成路径清空上次纹理、关闭 video_frame_active；渲染回调再次检查，不接受受限时排队的旧帧。
- [x] 5.4 GridRuntime 逐 session enforce_privacy，按 tile 决定 host 可见性；普通格子照常刷新。保护失败卸载对应会话/表面，不停止其它条目。
- [x] 5.5 GREEN：宿主/网格契约、合成纹理与窗口状态测试；覆盖全屏、弹窗关闭和布局轮询。
- [x] 5.6 提交：`feat: conceal protected native and composited video surfaces`。

## Task 6: PIN/设置窗口与应用接线

**Files:** 新建 privacy-window.slint、desktop/privacy/window.rs；修改 main-window.slint、app.rs、lib.rs、shortcut.rs、设置语言映射；新增 privacy_window_contract.rs。

- [x] 6.1 RED：渲染真实 Slint 窗口，验证四位输入、确认输入、取消、按钮门控、日程行与受保护名单控件；PIN 不触发播放器快捷键。
```rust
let window = PrivacyWindow::new().unwrap(); // initialize software test backend first
window.set_mode(1); // 0 setup, 1 authenticate, 2 settings, 3 change PIN
window.set_busy(true);
assert!(!window.get_submit_enabled());
window.set_busy(false);
window.set_pin("0123".into());
assert!(window.get_submit_enabled());
window.set_pin("１２３４".into());
assert!(!window.get_submit_enabled());
```
- [x] 6.2 独立窗口提供设置 PIN/验证 PIN/已授权设置三种页面。星期选择、HH:mm、规则增删、名单移除和改 PIN 均走服务接口；取消不变更策略。
- [x] 6.3 主菜单与工具栏“一键隐私模式”，状态说明自动/手动及接管时间；新增无默认按键的 ShortcutAction。当前、播放列表、历史的保护修改捕获稳定目标后验证 PIN，不依赖稍后可能变化的索引。
- [x] 6.4 初始化阶段先加载策略，再向全部 controller/grid session 注入；计时、窗口重新激活与播放器 poll 中统一应用保护。设置授权被周期/失焦撤销后清空输入和名单文本。
- [x] 6.5 统一投影当前标题、状态、轨道/字幕、历史、最近打开、播放列表和网格标签。受保护时只显示泛化占位；已授权设置窗口是唯一局部例外，不放行视频。
- [x] 6.6 正常关闭/更新前 flush 隐私状态，保存失败阻止降低保护或丢失安全状态的重启。既有“无痕记录”开关含义不变。
- [x] 6.7 GREEN：窗口契约、入口/快捷键测试、默认整仓测试、runtime 编译；软件渲染查看不透明隐私占位与 PIN 页，无真实影片/PIN。
- [x] 6.8 提交：`feat: integrate one-click privacy controls and settings`。

## Task 7: 端到端安全回归与交付

**Files:** 隔离 QA 脚本/测试扩展、文档站中英文隐私说明；保持发布版本和 tag 不变。

- [x] 7.1 扩展现有 TEST-ONLY QA 控制，测试专用时钟/虚构 PIN/媒体只在测试 feature 中启用；正式编译验证没有时钟/PIN 绕过入口。
- [x] 7.2 用明确有颜色、正在播放的样例验证进入时段前确有画面，再验证 pause/mute/native-visible/合成帧。多宫格一受保护、一普通；核对普通条目的 position 继续增加。
- [x] 7.3 覆盖手动关闭维持到次日开始、退出重启、日程结束不自动播放、错误 PIN 冷却、弹窗/全屏、EOF 下一项、历史和命令行绕过。
- [x] 7.4 Windows 本机、macOS ARM/Intel 与 Linux 原生验收分别留证据；当前不能执行的平台标明未验收，不用 mock 代替真实隐藏证据。macOS ARM/Intel 与 Linux X11 已由 CI run 37901280895 补齐（`feat/privacy-native`，`scripts/test-privacy-native-unix.ps1`），Wayland 仍未实测。
- [x] 7.5 执行并记录：
```powershell
cargo fmt --all --check
cargo test --workspace -j 2
cargo test -p yoyo-mpv --features mpv-runtime -j 2
cargo build -p yoyovideo-desktop --features mpv-runtime -j 2
git diff --check
```
- [x] 7.6 文档明确应用内保护边界、下一周期语义、PIN 不明文、恢复不自动播放；不声称加密或系统级防窥。
- [x] 7.7 核对所有已批准需求、旧 8 个文件的哈希、无 PIN/私钥泄露、提交范围；不自动推送、合并或改发布 tag。

## 接线实施细节

- DesktopRuntime 保留原 new 签名，新增配置入口注入服务；run 在初始播放前配置。DesktopController 通过 session setter 传入 Arc<dyn PlaybackAccess>；grid factory 在 open 前传入同一策略。
- 统一执行保护函数先设置 native permit 和清空 Slint Image，再 session.enforce_privacy；poll、渲染、窗口激活和所有命令入口调用。UI projection 接受 restriction 判定，保护中用“受保护内容 / Protected content”，不替换持久化原值。
- privacy-window 沿用 Slint 编译入口；mode 0/1/2/3，pin/confirmation/busy/submit-enabled 属性，submit/cancel/save-schedule/remove-protection/change-pin 回调。密码 LineEdit 使用 password 输入类型，主播放器不接收其按键。
- window.rs 保留 AuthGrant 和选定 MediaLocator（不是列表索引）；mpsc 后台结果经主 UI poll 消费。失焦/close 调 cancel_authorization，清除 pin/confirmation 与名单模型；新周期由 epoch 检查撤销。
- 主窗口提供 privacy-enabled、privacy-blocked、privacy-status 和 toggle-privacy/open-privacy-settings/protect-current/protect-playlist/protect-history 回调；稳定 locator 从原始模型查取，绝不从脱敏字符串重建。
- 将输出截图、复制/定位路径入口纳入统一受限检查；普通条目继续原行为。退出和 updater restart 都先 flush，失败显示泛化错误并取消退出。
- Task 7 建立独立 privacy-qa feature，而不是复用 updater QA 免密逻辑；不含该 feature 时不读 privacy QA 环境变量。Windows 原生验证用隔离目录及彩色运动样例，记录保护前后输出与普通网格进度，未实测平台不写“通过”。

## 实施记录（2026-10-09）

- 已实现策略、PIN/持久化、播放准入、真实表面许可、独立设置窗口和主应用所有入口。
- 后续补充：主窗归属检查防止隐藏 PIN 窗口成为视频宿主父窗；允许的新媒体加载先清除旧保护画面并解除旧暂停；mpv STOP/REPLACE 不再被当作 EOF。
- Windows 原生 13 项、4 次进程启动通过；真实 GL 表面遮挡与生命周期测试通过；软件渲染检查并修正了暗色复选框文字。
- 测试构建标记 privacy_qa，正式打包验证器拒绝带该标记的构建。
- macOS ARM/Intel 与 Linux X11 的原生验收在 CI 完成：run 37901280895（`feat/privacy-native`，squash 后的尖端提交 `7636633`），三个平台各 10 项通过、4 次进程启动；失败前先修正了「推进时钟越过冷却时落到了限制时段之外」的驱动假设。
- 详见 docs/testing/privacy-mode-acceptance.md；未实测平台（Wayland、Windows Setup 首次安装）明确标注。

## 自检

- Spec 1/3/4 → Task 1/4/6；Spec 5/9 → Task 2/4；Spec 6/7 → Task 3/5；Spec 8 → Task 6；Spec 10/11 → 全部模块边界与 Task 7。
- 手动覆盖、PIN、播放准入和真正隐藏是同一功能链，不拆成可绕过的独立开关。
- 接口调整必须先更新本计划中的消费者名称，先确认失败测试再实现，不允许只测试源文件是否包含某行代码。
- 按用户已选的串行方式执行，不再询问代理执行方式；没有子代理任务。
## 交付验证汇总

- 默认整仓 379 通过 / 1 个原有条件测试跳过；mpv-runtime 46 通过；桌面 runtime 210 通过 / 2 个条件测试跳过。
- Windows 原生最终 13 项、4 次进程启动通过：.cache/privacy-native-41bf44325bbf4f13902502018e1493da/report.json。
- Windows 实际 GL 绘制/保护/释放通过；UI 软件渲染已目视核对。
- 文档站 4 个测试及构建通过；打包测试拒绝带 privacy_qa 的正式包。
- 普通 runtime 二进制已恢复，build-info 确认两种 QA feature 均为 false。
- macOS ARM/Intel 与 Linux X11 原生验收通过：CI run 37901280895，各 10 项、4 次进程启动；Linux 仅覆盖 X11/Xvfb，Wayland 未实测。
- 既有 8 个未提交文件哈希保持不变；本次隐私工作没有重发版本、没有改动已发布 tag。
