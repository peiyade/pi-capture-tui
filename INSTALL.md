# 安装指南

## 快速安装

```bash
# 克隆仓库
git clone https://github.com/peiyade/pi-capture-tui.git
cd pi-capture-tui

# 运行安装脚本
./install.sh
```

安装脚本会自动：
- 检测您的 shell (fish/zsh/bash)
- 构建 release 版本
- 安装到 `~/.local/bin`
- 添加 PATH 配置

## 手动安装

### 1. 构建

```bash
cargo build --release
```

### 2. 复制二进制文件

```bash
# 创建目录
mkdir -p ~/.local/bin

# 复制
cp ./target/release/pi-capture ~/.local/bin/

# 添加 PATH
export PATH="$HOME/.local/bin:$PATH"
```

### 3. 持久化 PATH

**Fish:**
```fish
set -Ux fish_user_paths $HOME/.local/bin $fish_user_paths
```

**Zsh:**
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

**Bash:**
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc  # Linux
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bash_profile  # macOS
source ~/.bashrc  # 或 ~/.bash_profile
```

## 卸载

```bash
./install.sh uninstall
```

或手动：
```bash
rm ~/.local/bin/pi-capture
```

## 使用

```bash
pi-capture
```

## 配置

首次运行会自动创建配置文件：

- **macOS:** `~/Library/Application Support/pi-capture/config.yaml`
- **Linux:** `~/.config/pi-capture/config.yaml`

编辑配置文件添加 API key：
```yaml
ai:
  api_key: "your-api-key-here"
```

或使用 `.env` 文件：
```bash
echo "STEPFUN_API_KEY=your-key" > .env
```

## 依赖

- Rust 1.70+ (仅构建时需要)
