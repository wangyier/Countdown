# 我要第一个溜 · 下班倒计时

Windows 桌面挂件，Rust + Tauri v2 + 原生 HTML/CSS/JavaScript，无前端运行依赖。计薪功能不在当前范围。

## 本地预览

安装 Node.js 后，在项目根目录执行，无需 `npm install`：

```powershell
npm run preview
```

访问 <http://127.0.0.1:4173>。浏览器会显示预览标记，设置只存入浏览器本地存储；托盘、窗口置顶、系统通知、自启动和真实 IP 显示需运行桌面版。

## 桌面开发与打包

需要 Windows、Rust MSVC 工具链、Visual Studio C++ Build Tools、Windows SDK 和 WebView2 Runtime，参见 [Tauri Windows 环境要求](https://v2.tauri.app/start/prerequisites/#windows)。首次构建需要连接 crates.io 下载依赖。

在项目根目录执行：

```powershell
# 只需安装一次 Tauri 2 CLI
cargo install tauri-cli --version "^2" --locked

# 本地调试
cargo tauri dev

# 运行全部 Rust 测试与前端逻辑测试
cargo test --manifest-path src-tauri/Cargo.toml
npm test

# 格式检查与正式打包
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo tauri build
```

NSIS 安装包输出到 `src-tauri/target/release/bundle/nsis/`。无前端构建步骤，Tauri 直接嵌入 `src/`。

本次 release 程序已编译成功，原始文件为 `src-tauri/target/release/countdown.exe`，另复制为 `artifacts/我要第一个溜.exe`，可直接运行。安装版尚未生成：打包工具在下载 NSIS 3.13 时连接被拒绝，需恢复该下载后重新执行 `cargo tauri build`。

若环境设置了 `CARGO_NET_OFFLINE=true` 且尚未缓存依赖，可在自己的 PowerShell 会话中执行 `$env:CARGO_NET_OFFLINE = 'false'` 后重试；代理应指向实际可连接的代理地址。

## 不依赖 Cargo 下载的核心检查

```powershell
npm test
New-Item -ItemType Directory -Path artifacts -Force | Out-Null
rustc --edition 2024 --test src-tauri/src/clock.rs -o artifacts/clock-tests.exe
./artifacts/clock-tests.exe
```

`clock.rs` 是桌面后端实际调用的纯时间计算模块。独立测试覆盖工作边界、午休、休息日、全天进度范围与无效时间。日期、节假日、持久化和 Tauri 集成测试仍需运行完整 `cargo test`。

## 应用图标

图标原始附件为根目录 `icon-1.png`，画面是蓝色像素风时钟，原图已含透明通道。`scripts/prepare-icon.py` 仅按 alpha 边界 `(468, 140, 940, 628)` 去掉透明留白，将 472×488 原始像素居中放入 512×512 透明画布，保存为 `src/assets/app-icon.png`；不重绘、不去除黑色指针、不改变配色。窗口品牌标记与浏览器 favicon 直接使用此 PNG。托盘 16/20/24/32 px PNG 由同一源图通过 Lanczos 缩放，`desktop.rs` 仍显式嵌入 32 px PNG。旧 SVG 路径只保留为新 PNG 的兼容包装，不再包含旧设计。

重新生成桌面 PNG、Windows 多尺寸 ICO、macOS ICNS 和托盘 PNG：

```powershell
npm.cmd run icons
```

生成需要 Node.js、现有 `cargo tauri`、Python 3 和 Pillow（本机已安装；新环境执行 `python -m pip install Pillow`），无需图片生成服务。`npm.cmd run icons` 每次先从原始附件准备 PNG 与托盘资源，再在临时目录生成全套图标，仅复制桌面文件，结束后自动清理移动端副产物。`src-tauri/build.rs` 显式监听 `icons` 目录变化，避免仅替换资源时复用旧的 Windows 图标链接缓存。

只编译便携 release（不下载 NSIS）：

```powershell
$env:CI = 'true'
cargo tauri build --no-bundle -- --offline
```

`CI` 若由外部环境设置为 `1`，需改为 Tauri CLI 接受的 `true`。编译后将 `src-tauri/target/release/countdown.exe` 复制到 `artifacts/我要第一个溜.exe`；若目标正被运行中的应用锁定，应另存新文件名，不结束用户进程。已运行的旧进程需用户自行退出并启动新版本后才会显示新托盘图标，资源管理器也可能缓存旧 EXE 图标。

## 使用与配置

- 桌面版可在标题、倒计时文字和卡片空白区域按住鼠标左键移动窗口；按钮、设置表单和提示信息不触发拖动。图钉切换置顶（默认关闭），齿轮打开设置。已有配置中的手动选择会保留。浏览器预览不能拖动 Windows 窗口。
- 首页展示下班倒计时、掼蛋倒计时和今日进度，不显示上班/午休/下班时间表；具体时间在齿轮设置弹窗中修改。
- 窗口聚焦时按 Esc 隐藏；设置弹窗内先关闭弹窗。托盘左键显示/隐藏，右键可设置、置顶或退出。关闭窗口也会隐藏到托盘。
- 鼠标悬停托盘图标显示 `HH:MM:SS` 倒计时，每秒更新；移入图标或保存时间设置后立即刷新，挂件隐藏时也有效。工作日到配置的下班时间后移除下班整行，13:15 起移除掼蛋整行，两项都结束后仅显示“我要第一个溜”，无空行或结束文案。休息日保持原有休息提示。此功能需在桌面版使用，浏览器预览不包含系统托盘。
- 午休只暂停工时进度，下班倒计时继续。下班后归零，休息日显示“休息日 / 充能中”。
- 新增“掼蛋倒计时”，工作日目标固定为当天 11:45，附文案“速速吃饭，准备掼蛋”；到点归零，挂件在 11:45 至 13:15 前显示“掼蛋时间到！”，13:15 起改为“今日已结束”；托盘则在 13:15 起移除掼蛋整行。该目标和提示结束时间不随午休设置改变，次日自动重置；休息日不倒计时。
- 开机自启动默认开启，启动时尝试同步系统状态，设置中可关闭；系统操作失败会提示。退出需使用托盘“退出应用”。
- 多开应用会唤起已有窗口，避免重复托盘和提醒。

配置目录为 `%LOCALAPPDATA%/com.administrator.countdown/`：

```json
{
  "start_time": "09:00",
  "end_time": "17:30",
  "lunch_start_time": "11:45",
  "lunch_end_time": "13:15",
  "autostart_enabled": true,
  "always_on_top": false
}
```

保存为 `config.json`。要求上班 < 午休开始 < 午休结束 < 下班，不支持跨日班次。应用会先写临时文件并刷新，再替换配置文件；保存失败时尝试恢复系统设置。

内置 2026 年[官方放假区间](https://www.beijing.gov.cn/zhengce/zhengcefagui/202511/t20251104_4258873.html)，文件为 `src/data/holidays.json`。考勤按周一至周五及当月最后一个周六，再排除放假日期；国家额外调休补班不纳入。需要更新年度时，复制同结构的完整日历到配置目录的 `holidays.json`，修改 `years` 和 `ranges`，重启生效。未配置的年份会提示，并暂停外卖提醒。

外卖提醒在工作日 10:35 这一分钟触发，不补发；提醒日期记录在 `reminders.json`，重启不重复尝试。[Tauri 通知文档](https://v2.tauri.app/plugin/notification/)说明 Windows 通知需在安装版中验证，开发运行可能显示 PowerShell 名称和图标。

## 图标替换验证记录（2026-11-11）

- 已替换全部桌面 PNG、ICO、ICNS，并生成 16/20/24/32 px 托盘专用 PNG；已查看 128 px 主图与 16 px 托盘图。
- `npm.cmd test` 的 11 项前端测试、`cargo test --manifest-path src-tauri/Cargo.toml --offline` 的 11 项 Rust 测试、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 均通过；未新增纯资源测试。
- `CI=true cargo tauri build --no-bundle -- --offline` 成功。发现并修复图标资源缓存未刷新的问题后重新构建，使用 Windows 资源 API 读取最终 EXE，确认 6 个 RT_ICON 图像与新 ICO 逐字节相同，并确认托盘专用 PNG 解码后的 RGBA 数据已嵌入 EXE。
- 已覆盖 `artifacts/我要第一个溜.exe`，与 release 原文件逐字节一致，大小 4,892,160 字节；SHA-256：`a7a894a61bbe022f3ee93a249d1be8d405a35ca03ef0566d810a2f6238084cfa`。未终止用户进程。
- 本轮交付为便携 EXE，未生成安装包；未重启现有应用，实际 Windows 托盘和不同 DPI 下的外观仍需用户运行新版确认。已清理本轮生成的移动端资源与临时目录，保留桌面图标源和可复用脚本。

## 托盘提示精简验证（2026-11-11）

- 托盘实际调用 `tooltip_text` 生成完整文本。新增测试通过真实考勤计算后逐字断言最终字符串，确认移除整行而非替换成结束文案，同时避免尾随换行。
- 默认工作日：13:14:59 仍含“掼蛋时间到！”，13:15:00 起仅保留下班倒计时；17:29:59 显示“距离下班 00:00:01”，17:30:00 起仅显示“我要第一个溜”。
- 已验证自定义 18:00 下班、12:00 提前下班、午休调整不改变固定 13:15 掼蛋截止、午夜重置，以及普通周六、周日、法定节假日和月末周六。休息日提示保留。
- 完整 Rust 测试 14 项、前端测试 11 项和 Rust 格式检查均通过。`CI=true cargo tauri build --no-bundle -- --offline` 成功。
- 已覆盖 `artifacts/我要第一个溜.exe`，与 `src-tauri/target/release/countdown.exe` 逐字节一致，Windows PE 格式有效，大小 4,892,672 字节，SHA-256：`32375e49906792051e132550d51bdec472597fd3140c9a3c1858bf6d67814d18`。未终止用户进程，未修改挂件 UI。未进行 Windows 托盘悬浮气泡的人工视觉验证。

## 用户原图图标交付（2026-11-11，当前版本）

- 已实际读取 `icon-1.png`，保留蓝色像素时钟设计和原始 alpha。512×512 最终源中 `(20, 12, 492, 500)` 区域与原图有效区域逐字节一致；原图保留不变。
- 更新应用 PNG、6 尺寸 ICO（16/24/32/48/64/256）、ICNS、全部桌面 Logo、16/20/24/32 px 托盘 PNG、页面品牌与 favicon。已检查所有 PNG 为正方形 RGBA、alpha 范围 0–255，并查看 16/20/24/32/128 px 在深浅背景上的效果；小尺寸保留轮廓与指针，部分像素细节随缩放减少。
- 生成脚本从用户原图开始处理，重复生成源图和托盘图的 SHA-256 不变；不再使用旧设计。Pillow 13.0.0 已在本机 Python 3.12 安装。
- 未修改 `desktop.rs`，保留 tooltip 精简逻辑。11 项前端测试、14 项 Rust 测试（含 tooltip 精确边界）和 Rust 格式检查通过。
- `CI=true cargo tauri build --no-bundle -- --offline` 构建成功。通过 Windows 资源 API 读取实际 release EXE 的 RT_GROUP_ICON/RT_ICON，6 个图像与新 ICO 内容逐字节一致；32 px 新托盘 RGBA 数据也存在于 EXE 中。
- 已覆盖 `artifacts/我要第一个溜.exe`，与 `src-tauri/target/release/countdown.exe` 逐字节一致。大小 5,174,272 字节；SHA-256：`79b188ba9da4a37a743078527bd45ad921493e199eb154b53dcab70aab0410cb`。本记录取代上方历史版本的交付哈希。
- 本次交付为便携 EXE，未构建 NSIS 安装包；未启动或重启用户应用，真实托盘、Windows 图标缓存和各 DPI 外观仍需运行新版确认。旧进程需退出后再启动新版。临时审阅图、核验脚本和构建日志已清理。

## 本轮验证记录

- 通过：完整 `cargo test --manifest-path src-tauri/Cargo.toml --offline` 的 11 项 Rust 测试，以及 `npm.cmd test` 的 11 项前端测试，涵盖掼蛋倒计时、13:15 提示截止、考勤和拖动事件；已执行 Rust 格式化。
- 执行环境已恢复：默认 WindowsApps PowerShell 入口在当前沙箱启动时报 `CreateProcessAsUserW` 错误 5；显式指定 `shell: "cmd.exe"`、`login: false` 后 Cargo、Rust、Tauri CLI 和测试均可运行。项目 `AGENTS.md` 已记录此选择。
- release 构建成功：`cargo tauri build` 已完成优化编译并输出 `src-tauri/target/release/countdown.exe`。已验证交付副本是 x64 Windows PE 文件；exe 为 `artifacts/我要第一个溜.exe`。最终命令因 NSIS 3.13 下载连接被拒绝而返回失败，安装包尚未生成。
- 视觉检查未完成：browser-skill 无日志目录写入权限；电脑操作工具无法读取最小化的 Chrome 窗口。
- 待实机验收：360×480 显示与拖动、托盘菜单与秒级提示、Esc 隐藏恢复、设置保存及重启恢复、置顶、自启动和安装版通知。
