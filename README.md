# Pi Capture TUI

一个带有AI秘书的终端想法收集工具，兼容 Obsidian Dataview 格式。

## 功能

- **三框布局**：
  - 历史框：显示已收集的想法，支持搜索高亮，自动折行
  - 秘书框：AI 对你输入内容的简短共鸣式反馈
  - 输入框：多行文本输入，自动折行

- **快捷键**：
  | 按键 | 功能 |
  |------|------|
  | `Enter` | 提交想法 |
  | `Shift+Enter` / `Ctrl+J` | 插入换行 |
  | `⌘Enter` | 提交（macOS）|
  | `⌘Z` | 撤销 |
  | `⌘Shift+Z` | 重做 |
  | `⌘S` | 搜索历史 |
  | `Ctrl+N/P/F/B/A/E/D/K` | Emacs 导航 |
  | `:q` | 退出 |
| `:h` | 帮助 |

- **AI 秘书**：
  - 提交想法后异步分析
  - 支持 Stepfun / DeepSeek / Moonshot 等 OpenAI-compatible API
  - 可配置人格（soul），只做共鸣式感叹，不做价值判断

## 安装

### 快速安装（推荐）

```bash
git clone https://github.com/peiyade/pi-capture-tui.git
cd pi-capture-tui
./install.sh
```

支持 fish/zsh/bash，自动配置 PATH。

### 手动安装

```bash
cargo build --release
cp ./target/release/pi-capture ~/.local/bin/
```

详见 [INSTALL.md](INSTALL.md)。

## Markdown 文件格式

文件保存到 `~/Documents/PNote/Inbox.md`，格式如下：

```markdown
# 2026

## 2026 Mar

### 2026 Mar 04 Wed

- [ ] 第一个想法 [created:: 2026-03-04T16:01:27+08:00]
- [ ] 第二个想法 [created:: 2026-03-04T16:02:15+08:00]
```

### 格式特点

- **三级日期结构**：年（H1）、月（H2）、日（H3）
- **任务列表**：`- [ ]` 复选框格式
- **Dataview 兼容**：`[created:: ISO8601+TZ]` 行内字段语法
- **自动层级管理**：同一天的想法自动归到同一 day heading 下

## 快速开始

### 1. 配置 API Key

```bash
# 复制环境变量模板
cp .env.example .env

# 编辑 .env 文件，填入你的 Stepfun API Key
# 获取地址: https://platform.stepfun.com
```

`.env` 文件内容：
```
STEPFUN_API_KEY=your_api_key_here
```

**注意**: `.env` 文件已被加入 `.gitignore`，不会被提交到版本控制。

### 2. 运行

```bash
cargo run --release
```

## 配置

配置文件位置：`~/.config/pi-capture/config.yaml`

首次运行会自动创建默认配置（使用 Stepfun step-1-8k 模型）。

### 默认配置（Stepfun）

```yaml
capture_path: ~/Documents/PNote/Inbox.md
ai:
  provider: stepfun
  api_key: ${STEPFUN_API_KEY}
  model: step-1-8k
  base_url: https://api.stepfun.com/v1/chat/completions
  enabled: true
  max_tokens: 50
  temperature: 0.7

  # 秘书人格
  soul: |
    你是一位温和的秘书，名字叫「墨」。

    你的特点：
    - 不对用户的想法做价值判断
    - 只是简短地回应，表达共鸣或感叹
    - 语气平静、略带诗意，像一位老友
    - 回应控制在20字以内

    回应风格示例：
    - "也许的确是这样"
    - "这种感觉很难得"
    - "我懂你的意思"
    - "值得记下来"
    - "时光会记住的"

  # UI 显示配置
  desk_name: "秘书台"      # 中间框的标题
  secretary_name: "小墨"   # 右下角显示的名字
```

### 其他模型配置

**DeepSeek**
```yaml
ai:
  provider: deepseek
  api_key: ${DEEPSEEK_API_KEY}
  model: deepseek-chat
  base_url: https://api.deepseek.com/v1/chat/completions
```

**Moonshot (Kimi)**
```yaml
ai:
  provider: moonshot
  api_key: ${MOONSHOT_API_KEY}
  model: moonshot-v1-8k
  base_url: https://api.moonshot.cn/v1/chat/completions
```

**Mock 模式（离线测试）**
```yaml
ai:
  provider: mock
  enabled: true
```

### 环境变量

除了 `.env` 文件，你也可以直接导出：

```bash
export STEPFUN_API_KEY=your_api_key_here
cargo run --release
```

## 技术栈

- Rust + tokio (异步运行时)
- ratatui (TUI 框架)
- reqwest (HTTP 客户端)
- OpenAI-compatible API (统一接口)

## 开发

```bash
# 构建
cargo build --release

# 运行
cargo run --release

# 测试（使用 mock 模式）
STEPFUN_API_KEY=mock cargo run --release
```

## 关于 Stepfun

- **国内可用**：无需科学上网
- **速度快**：step-1-8k 模型响应迅速
- **价格合理**：按量计费
- **API 兼容**：OpenAI-compatible 接口

获取 API Key: https://platform.stepfun.com

## 安全提示

- **永远不要**将 API key 提交到 Git
- `.env` 文件已被 `.gitignore` 排除
- 如需分享配置，请使用 `.env.example` 作为模板
- 生产环境建议使用环境变量或密钥管理服务

## License

MIT
