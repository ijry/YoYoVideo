# 隐私模式验证记录

日期：2026-10-09。分支：`feat/privacy-mode`。

这是本次新增隐私功能的验证，不替代此前已经完成的 v0.0.1 更新器验收。没有推送、改 tag 或重新发布版本。

## Windows 原生验证

命令：

```powershell
cargo build -p yoyovideo-desktop --features privacy-qa -j 2
pwsh -NoProfile -File scripts/test-privacy-native-windows.ps1
```

测试只接受 `--build-info` 标明 `privacy_qa: true` 的二进制，并且要求独立的带标记夹具目录。时间修改仅作用于测试构建的注入时钟，不修改系统时间。控制消息和诊断 JSON 不含 PIN 数字或校验值。

已运行的记录：

- `.cache/privacy-native-41bf44325bbf4f13902502018e1493da/report.json`：最终 13 项通过，4 次真实进程启动。
- 红/蓝视频由 ffmpeg 生成；先证明本进程的视频窗口内有可见红色像素且播放位置持续增加，再验证暂停、静音和隐藏。像素探测先检查前台窗口及采样点归属，不采样其它应用窗口。
- 直接读取 libmpv 的 `pause`、`mute`、`idle-active` 标志，避免把界面状态误当成后端事实。
- 覆盖首次 PIN、PIN 授权名单/日程修改、进入受限时段时的实际隐藏、截图/恢复拒绝、弹窗/全屏、历史/最近/拖放拒绝不替换普通影片、EOF 拦截、跨周期及重启手动覆盖、解除不自动播放、逐格保护、音量保持、错误 PIN 冷却重启、损坏配置保持保护。
- 宿主窗口归属于主窗口，而不是提前创建的隐藏 PIN/设置窗口。
- 从受保护影片切换到允许的新影片时，先卸载旧画面，明确恢复新加载的播放，不继承旧影片的保护暂停。

最终重跑包含受保护命令行启动：在持久限制/冷却期间通过启动参数传入受保护样例，后端保持 idle，未加载或显示该影片。

## 真实 OpenGL 合成表面

```powershell
cargo test -p yoyovideo-desktop --features mpv-runtime --lib native_gl_decodes_resizes_and_tears_down -- --ignored --test-threads=1
```

Windows 本机 **通过**。实际解码红色帧并读取 GL 像素，验证尺寸改变，再开启保护：即使请求新的绘制尺寸也不提交图像、不调整目标纹理；最后确认渲染上下文正确释放。

这证明共享合成表面的保护门有效，但不能据此声称已经在 Wayland 或 macOS 的实际窗口系统上通过验收。

## 单元、界面及打包回归

- 日程：半开区间、星期、跨夜、区间合并、DST 重复/缺失时刻、手动覆盖及重启。
- PIN：真实 Argon2id、前导零、输入限制、原子配置、损坏/超大配置、持久冷却、后台验证不先放行、取消/失焦/新周期后的旧回复拒绝。
- 播放：先准入后变更队列、幂等暂停、保护静音与用户偏好分离、失败时停止卸载并保留恢复位置、切换清除旧画面、只把真正 EOF 当作自动连播。
- 界面：真实软件渲染测试显示红色图像后验证不透明占位；验证 800×600 空态及描边按钮；PIN 表单门控、授权模型清除、暗色设置窗口文字对比度和无默认绑定的快捷键。
- 打包：普通验证器同时拒绝 `updater_qa` 和 `privacy_qa` 测试构建；没有改变发布版本或签名密钥。

默认整仓回归：**379 通过，0 失败，1 个原有桌面条件测试跳过**（窗口离屏恢复冒烟）。`yoyo-mpv --features mpv-runtime`：**46 通过**；桌面 `mpv-runtime` 回归：**210 通过，2 个需显式启动窗口的测试跳过**。上述 OpenGL 条件测试已另行显式运行通过。

库级 Clippy 检查通过。桌面检查只放行原有的 `too_many_arguments`、`ptr_arg`、`unnecessary_cast`、`manual_clamp` 类别；core 只放行原有的派生默认实现、`next` 命名和构造赋值风格告警。未为本功能重构旧代码。严格 all-targets 检查还会命中旧 `drop_contract.rs` 的 `cloned_ref_to_slice_refs`，没有将它称为全量零告警。

日志均保存在 `.cache/privacy-*.log`。界面预览仅使用虚构数据：`.cache/privacy-settings-preview.png`。

## 尚未运行的平台验收

| 平台 | 本次隐私功能原生验收 |
| --- | --- |
| Windows x64 原生窗口 | 已运行并通过上述场景 |
| Windows OpenGL 合成表面 | 已运行并通过 |
| macOS Apple Silicon / Intel | 未在本次运行中实测 |
| Linux X11 / Wayland | 未在本次运行中实测 |

本次没有为验证而推送 CI 提交。后续在对应系统/CI 运行时，应同样先证明样例确实可见且正在播放，不能把黑屏或 mock 当作真实遮挡通过。

## 普通构建

已构建并恢复 `target/debug/yoyovideo-desktop.exe`（50,788,352 字节），使用 `mpv-runtime`、不带任何 QA feature。`.cache/privacy-production-build-info.json` 已确认 `mpv_runtime: true`、`privacy_qa: false`、`updater_qa: false`。可复查：

```powershell
cargo build -p yoyovideo-desktop --features mpv-runtime -j 2
./target/debug/yoyovideo-desktop.exe --build-info
# mpv_runtime: true, privacy_qa: false, updater_qa: false
```

本机构建一度遇到 D 盘空间不足；没有使用被拒绝的删除操作，转用 C 盘临时 target，关闭增量缓存和调试符号。既有安装器相关 8 个文件使用原始 SHA-256 快照核对，不纳入本功能提交。
