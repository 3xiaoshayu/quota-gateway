# Quota Gateway

把 Codex、Claude、Antigravity 的订阅账号变成本机 API，再一键接给 Codex、Claude Code、Claude Desktop、OpenCode、ZCode 这些智能体。Windows 桌面程序，Tauri 2 + React，内核是 [CLIProxyAPI](https://github.com/router-for-me/CLIProxyAPI)。

这是 [EasyCLIProxyAPI](https://github.com/router-for-me/EasyCLIProxyAPI) 的私人精简分支。上游没有声明开源许可证（GitHub API 里 `license` 为空），所以这个仓库只作私有备份和自己使用：**不要设为公开，不发布、不分发安装包**。若上游日后加上许可证，再按其条款处理。

## 只留四页

| 页面 | 做什么 |
| --- | --- |
| 首页 | 启动 / 停止 / 重启内核；OpenAI、Claude、Gemini 三种兼容地址和第一把本机 API Key，点一下复制 |
| OAuth | Codex / Claude / Antigravity 网页授权；认证文件的启用、停用、上传、删除；每个账号的额度 |
| 智能体配置 | 检测本机的 Codex、Claude Code、Claude Desktop、OpenCode、ZCode，选模型、写配置、启动 |
| 高级设置 | 端口、局域网、代理、路由策略、重试、日志、开机自启、关闭行为 |

## 相比上游删掉的

新手模式、版本管理与自动更新、API 接入（API Key 上游聚合）、使用记录与计费、Codex 会话管理、模型别名页、Kimi / xAI 登录、OpenClaw / Hermes / Pi / Kimi Code / Grok Build / DeepSeek Harness 智能体、多语言、日间主题、联系我们 / 赞助入口、GitHub 发布流水线。

内核仍然从 `cpa-core/` 里随包附带的压缩包安装，不再联网检查新版本。

## 和 Quota Switcher 的分工

[Quota Switcher](https://github.com/3xiaoshayu/quota-switcher) 管的是**官方客户端**：把账号写进官方 Codex / Cursor / Antigravity IDE，看额度、手动切号。Quota Gateway 管的是**API**：把同一批订阅账号挂在本机 `127.0.0.1:8318` 上，给命令行智能体用。两边界面用同一套深色设计，可以一起开。

默认端口是 8318，和原版 EasyCLIProxyAPI 的 8317 错开，过渡期两个都能跑。

## 开发

```powershell
bun install
bun run check          # tsc
bun test               # 前端单测
cd src-tauri; cargo test; cd ..
.\build.ps1            # 编译并整理到 bin-work\
.\run.ps1              # 运行 bin-work\Quota Gateway.exe
```

`build.ps1` 会把 `cpa-core\CLIProxyAPI_<版本>_windows_amd64.zip` 随包放进 `bin-work\cpa-core\`（版本号见 `core-version.txt`）；本地没有这个压缩包时从 GitHub Releases 下载一次并校验 SHA-256。

配置和账号文件都放在可执行文件旁边的 `cpa-core\` 里，和原版互不影响。
