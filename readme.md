# 我要第一个溜 · 下班倒计时桌面挂件

当前范围：暂不实现计薪功能，包括月薪配置、时薪/秒薪计算、收入展示、金额插值和薪资隐私开关。后续如需恢复，另行纳入开发范围。

## 当前实现进度（2026-11-12）

- 已编写：深色透明挂件、秒级倒计时、有效工时进度、四种工作状态、设置弹窗及校验、Rust 配置持久化、托盘菜单与悬停倒计时、置顶、快捷隐藏、每天外卖提醒、自启动和 IP 末段显示。
- 已通过完整 Rust 测试 14 项与前端测试 11 项。WindowsApps PowerShell 入口无法在当前沙箱启动，显式使用 `cmd.exe` 后 Cargo、Rust 与 Tauri CLI 正常运行。NSIS 安装包输出到 `src-tauri/target/release/bundle/nsis/`。Windows 系统通知与自启动仍需安装版实机验收。
- 视觉检查尚未完成：browser-skill 无法创建日志文件，电脑操作工具无法读取最小化的 Chrome 窗口。
- 开发与打包步骤见 [README.tauri.md](README.tauri.md)。可运行 `npm run preview` 查看浏览器预览；预览不会启用托盘、置顶、通知或系统自启动。

技术栈：Rust + Tauri v2 + 原生 HTML/CSS/JavaScript，不使用 Vue/React 等重型前端框架。

主要功能：

下班倒计时：默认 9:00-17:30，可在设置中修改。

掼蛋与午休共用时间：倒计时至午休开始，午休期间显示“掼蛋时间到！”，午休结束后显示“今日已结束”。

午休：11:45-13:15，工时进度暂停，距离下班的倒计时继续。

提醒外卖：用户可在设置中开启或关闭、自定义提醒时间，默认开启并设为 10:35。通过 Windows 系统通知提醒，每天最多一次，错过不补发，保存后立即生效。

本地ip显示：192.168.0.xxx（仅显示xxx）

鼠标移动到托盘图标显示倒计时，精确到秒

支持开机自启动，默认开启，并提供设置开关。

## 新电脑开发准备（Windows）

项目可放在任意有读写权限的目录，不要求沿用原电脑盘符或用户名。源码和图标资源均随仓库保存；`.cache/`、`artifacts/`、`src-tauri/target/` 和 `.alma-snapshots/` 是本机缓存、产物或快照，不需要从旧电脑复制。

1. 安装 Git、Node.js 当前 LTS（含 npm）、Rust stable 的 MSVC 工具链，以及 Visual Studio 2022 Build Tools 的“使用 C++ 的桌面开发”工作负载和 Windows SDK；确认已安装 Microsoft Edge WebView2 Runtime。Rust 使用 edition 2024，请使用最新 stable。详细要求见 [桌面开发与打包](README.tauri.md#桌面开发与打包)。
2. 克隆仓库或复制源码后，打开项目根目录终端。以下命令使用 PowerShell；若 npm 脚本被执行策略拦截，可直接使用 `npm.cmd`。
3. 首次联网安装 Tauri 2 CLI，并按锁文件下载 Rust 依赖。项目目前没有 npm 依赖，无需 `npm install`；应保留并使用 `src-tauri/Cargo.lock`。

```powershell
node --version
cargo --version
rustup show active-toolchain
cargo install tauri-cli --version "^2" --locked
cargo fetch --manifest-path src-tauri/Cargo.toml --locked

# 浏览器预览：http://127.0.0.1:4173，Ctrl+C 停止
npm.cmd run preview
```

预览不提供托盘、系统通知、自启动和真实 IP。停止预览后，可在项目根目录执行：

```powershell
# 检查与桌面调试
npm.cmd test
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo tauri dev

# 退出调试后，构建便携 EXE（不生成安装包）
cargo tauri build --no-bundle -- --locked

# 构建 NSIS 安装包，需要联网下载相应打包工具
cargo tauri build -- --locked
```

便携程序位于 `src-tauri/target/release/countdown.exe`，安装包位于 `src-tauri/target/release/bundle/nsis/`。首次构建需要可用的 Cargo 和打包工具下载网络；不要在未缓存依赖的新电脑使用 `--offline`。若继承了 `CARGO_NET_OFFLINE=true`，可在当前 PowerShell 会话设置 `$env:CARGO_NET_OFFLINE = 'false'`；若设置 `CI`，请使用字符串 `true`，不要使用 `1`。无需复制旧电脑的代理、Cargo 缓存或个人工具路径。

已有图标可直接构建。只有需要重新生成图标时，才需额外安装 Python 3（`python` 命令可用）及 Pillow：先执行 `python -m pip install Pillow`，再执行 `npm.cmd run icons`。脚本从根目录 `icon-1.png` 生成资源，详见 [应用图标](README.tauri.md#应用图标)。

应用首次启动会按默认配置尝试启用开机自启动，可在设置中关闭。个人配置保存在当前用户的 `%LOCALAPPDATA%/com.administrator.countdown/`，由系统目录 API 定位，不依赖旧电脑用户名；迁移个人设置时自行备份、复制该目录的配置文件。`com.administrator.countdown` 是应用标识符，不是 Windows 账号路径，修改它会改变配置目录。项目运行不需要 API 密钥或 `.env`；真实 `.env`、私钥、签名证书和 Cargo 凭据应只保留在本机，示例文件仅可包含占位值。Git 作者身份由开发者使用自己的真实信息自行配置，不属于运行或暂存前置条件。

# 任务目标：构建 Windows 桌面端

请基于 **Rust + Tauri v2**（前端使用轻量原生 HTML/CSS/JavaScript，零重型框架依赖）构建一个常驻 Windows 桌面边缘的轻量、半透明毛玻璃悬浮小挂件。

---

## 1. 核心功能规范

### 1.1 核心算法与 IPC 接口 (Rust 后端)

- **配置持久化**：
    - 数据模型包含：`start_time`（上班时间，默认 "09:00"）、`end_time`（下班时间，默认 "17:30"）、`lunch_start_time`（午休开始时间，默认 "11:45"）、`lunch_end_time`（午休结束时间，默认 "13:15"）、`autostart_enabled`（开机自启动，默认 `true`）、`always_on_top`（窗口置顶，默认 `false`）。
    - 每日有效工时为上下班间隔减去午休时长的派生值，默认 7 小时，用于计算工时进度，不作为独立可编辑配置。当前无需计算当月总出勤天数。
    - 暂不支持夜班或跨日班次，配置必须满足 `start_time < lunch_start_time < lunch_end_time < end_time`，避免无效工时与除零。
    - 数据通过 `serde_json` 存储于系统的本地应用数据目录（`AppLocalData`）。
- **Tauri Commands (IPC)**：
    - `get_config()`: 获取当前工作时间与自启动配置。
    - `save_config(config: WorkConfig)`: 保存并持久化新配置。
    - `get_countdown_stats()`: 根据当前本地时间，计算并返回：
        - `progress_percent` (今日工时完成百分比 0.0 - 100.0)
        - `remaining_seconds` (距离下班剩余秒数)
        - `guandan_remaining_seconds` (距离午休开始的剩余秒数，到点归零)
        - `guandan_ready` (午休期间为 true，控制掼蛋到点提示)
        - `status` (`before_work` / `working` / `lunch_break` / `after_work`，用于前端状态展示)
- **倒计时与时间边界**：
    - 使用系统本地日期与时间，每天按相同作息计算，午夜切换到新一天。
    - 上班前：工时进度为 0，倒计时为距离当天下班的实际秒数。
    - 工作期间：仅累计已过去的有效工作时间，计算工时进度。
    - 午休期间：工时进度暂停，距离下班的倒计时继续；午休结束后恢复累计。
    - 下班后：工时进度为 100%，倒计时归零。

### 1.2 系统级窗口行为与托盘集成 (Rust 后端)

- **无边框透明窗口**：
    - 窗口尺寸：宽 360px，高 480px，支持窗口无边框（`decorations: false`）、透明背景（`transparent: true`）、置顶开关（`alwaysOnTop`）。
- **拖拽支持**：桌面版在卡片非交互区域按住鼠标左键可自由拖动，通过 `getCurrentWindow().startDragging()` 显式触发；按钮、设置表单与提示信息不触发拖动。浏览器预览不支持移动系统窗口。
- **系统托盘 (System Tray)**：
    - 图标常驻右下角任务栏通知区。
    - 鼠标悬停显示距离下班的 `HH:MM:SS`，每秒更新；移入图标、保存时间设置后立即刷新，挂件隐藏后继续更新。每天到配置的下班时间后移除下班整行，午休结束起移除掼蛋整行，两项都结束后只显示应用名。
    - 左键单击：切换挂件的显示/隐藏（Show/Hide）。
    - 右键菜单：包含「显示/隐藏窗口」、「窗口置顶切换」、「配置设置」、「退出应用」。
- **老板键/快捷隐藏**：窗口聚焦时按 `Esc` 立即隐藏到托盘；设置弹窗打开时先关闭弹窗，再次按下隐藏挂件。不注册全局快捷键。
- **系统通知与自启动**：
    - 每天 10:35 发送一次外卖提醒，使用 Windows 系统通知。
    - 默认开启开机自启动，设置中可关闭；保存后同步实际系统自启动状态。

### 1.3 前端界面设计 (Webview2 / HTML+CSS+JS)

- **视觉风格**：
    - 现代深色磨砂亚克力/毛玻璃（Glassmorphism）风格：`backdrop-filter: blur(16px)`，圆角卡片，细微高光描边，柔和环境阴影。
    - 整体紧凑精致，适合作为桌面小组件常驻。
- **倒计时展示**：
    - 首页不显示上班/午休/下班时间表，具体时间在齿轮设置弹窗中查看和修改。
    - 大字号清晰展示距离下班的 `HH : MM : SS`。
    - 环形或条形进度条，流光渐变填充。
    - 状态边界、恢复显示及配置变更后重新同步后端数据，确保倒计时和工时进度准确。
- **实用小细节**：
    - 点击齿轮按钮唤起弹窗，支持就地修改上下班时间、午休时间和开机自启动开关，通过校验后调用 `save_config`，立即重新计算并更新界面。

---

## 2. 推荐工程目录结构

```

├── src/
│   ├── index.html        # 悬浮窗主体结构与设置弹窗
│   ├── styles.css        # 毛玻璃、字体、流光动效样式
│   └── main.js           # IPC 调用、倒计时更新、设置交互
├── src-tauri/
│   ├── Cargo.toml        # 依赖 chrono, serde, serde_json, tauri 等
│   ├── tauri.conf.json   # 窗口透明度、尺寸、托盘与权限配置
│   ├── icons/            # 托盘与应用图标
│   └── src/
│       ├── lib.rs        # 命令注册与生命周期管理
│       ├── main.rs       # 程序入口与托盘事件监听
│       ├── config.rs     # 配置文件 IO 与数据模型
│       └── calculator.rs # 倒计时与工时进度计算
```

## 3. 产出要求

1. 请按文件逐步输出完整的代码实现，重点包含 `Cargo.toml`、`tauri.conf.json`、`calculator.rs`、`lib.rs/main.rs` 以及前端 `index.html`、`styles.css`、`main.js`。
2. 时间计算应覆盖午夜切换、午休和上下班边界，避免负倒计时与无效进度。
3. 请附上本地调试（`cargo tauri dev`）和正式打包（`cargo tauri build`）的完整操作指令。

### 日期规则

每天按配置的上下班和午休时间计算，不判断周末、月末周六或节假日。无需维护日历文件或年度数据。

## 4. 首版实现约定

- `Esc` 采用仅窗口聚焦时立即隐藏；设置弹窗内先关闭弹窗。可后续按使用习惯调整。
- IP 只考虑私有 IPv4，优先 `192.168.0.x`，再选其他 `192.168.x.x`，最后为其他私有网段；同网段优先非 VPN/虚拟网卡名称。界面仅展示末段，找不到时显示 `—`。这是确定性优先规则，不保证识别所有 VPN 或物理网卡。
- 外卖提醒仅在用户设置的时间所在分钟内触发，应用需要运行；关闭提醒或晚于该分钟启动不补发。提醒尝试前持久化日期，重启、改时间或重新开启后，同一天不重复发送。Windows 通知需在安装版中做最终验证。
