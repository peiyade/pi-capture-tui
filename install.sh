#!/usr/bin/env bash
# Pi Capture TUI 安装脚本
# 支持 fish/zsh/bash

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 项目信息
BINARY_NAME="pi-capture"
REPO_NAME="pi-capture-tui"

# 检测 shell
 detect_shell() {
    if [ -n "$FISH_VERSION" ]; then
        echo "fish"
    elif [ -n "$ZSH_VERSION" ]; then
        echo "zsh"
    elif [ -n "$BASH_VERSION" ]; then
        echo "bash"
    else
        echo "unknown"
    fi
}

# 检测架构
detect_arch() {
    local arch=$(uname -m)
    case "$arch" in
        x86_64)
            echo "x86_64"
            ;;
        arm64|aarch64)
            echo "aarch64"
            ;;
        *)
            echo "$arch"
            ;;
    esac
}

# 检测平台
detect_platform() {
    local platform=$(uname -s)
    case "$platform" in
        Linux)
            echo "linux"
            ;;
        Darwin)
            echo "macos"
            ;;
        *)
            echo "$platform"
            ;;
    esac
}

# 获取安装路径
get_install_dir() {
    # 优先使用 ~/.local/bin
    if [ -d "$HOME/.local/bin" ]; then
        echo "$HOME/.local/bin"
    elif [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
        echo "/usr/local/bin"
    else
        # 创建 ~/.local/bin
        mkdir -p "$HOME/.local/bin"
        echo "$HOME/.local/bin"
    fi
}

# 添加到 PATH
add_to_path() {
    local install_dir="$1"
    local shell_type=$(detect_shell)
    local config_file=""

    # 检查是否已在 PATH 中
    if echo "$PATH" | grep -q "$install_dir"; then
        return 0
    fi

    case "$shell_type" in
        fish)
            config_file="$HOME/.config/fish/config.fish"
            mkdir -p "$(dirname "$config_file")"
            if ! grep -q "set -gx PATH" "$config_file" 2>/dev/null | grep -q "$install_dir"; then
                echo "set -gx PATH $install_dir \$PATH" >> "$config_file"
                echo -e "${BLUE}已添加 $install_dir 到 fish PATH${NC}"
            fi
            ;;
        zsh)
            config_file="$HOME/.zshrc"
            if ! grep -q "export PATH.*$install_dir" "$config_file" 2>/dev/null; then
                echo "export PATH=\"$install_dir:\$PATH\"" >> "$config_file"
                echo -e "${BLUE}已添加 $install_dir 到 zsh PATH${NC}"
            fi
            ;;
        bash)
            config_file="$HOME/.bashrc"
            if [ "$(uname -s)" = "Darwin" ]; then
                config_file="$HOME/.bash_profile"
            fi
            if ! grep -q "export PATH.*$install_dir" "$config_file" 2>/dev/null; then
                echo "export PATH=\"$install_dir:\$PATH\"" >> "$config_file"
                echo -e "${BLUE}已添加 $install_dir 到 bash PATH${NC}"
            fi
            ;;
        *)
            echo -e "${YELLOW}警告: 无法自动检测 shell，请手动添加以下到您的 shell 配置:${NC}"
            echo "  export PATH=\"$install_dir:\$PATH\""
            ;;
    esac
}

# 主安装流程
main() {
    echo -e "${BLUE}=== Pi Capture TUI 安装脚本 ===${NC}"
    echo ""

    # 检测环境
    local shell_type=$(detect_shell)
    local arch=$(detect_arch)
    local platform=$(detect_platform)

    echo -e "检测到环境:"
    echo "  Shell: $shell_type"
    echo "  架构: $arch"
    echo "  平台: $platform"
    echo ""

    # 检查是否是源码安装
    if [ -f "./target/release/$BINARY_NAME" ]; then
        echo -e "${GREEN}发现本地构建的二进制文件${NC}"
        SOURCE_BINARY="./target/release/$BINARY_NAME"
    elif [ -f "./target/release/pi-capture-tui" ]; then
        echo -e "${GREEN}发现本地构建的二进制文件${NC}"
        SOURCE_BINARY="./target/release/pi-capture-tui"
    else
        echo -e "${YELLOW}未找到本地构建的二进制文件，正在构建...${NC}"
        if ! command -v cargo &> /dev/null; then
            echo -e "${RED}错误: 未找到 cargo，请先安装 Rust${NC}"
            exit 1
        fi
        cargo build --release
        SOURCE_BINARY="./target/release/$BINARY_NAME"
    fi

    # 确定安装目录
    INSTALL_DIR=$(get_install_dir)
    echo -e "安装目录: ${BLUE}$INSTALL_DIR${NC}"

    # 安装二进制文件
    echo -e "${YELLOW}安装 $BINARY_NAME...${NC}"
    cp "$SOURCE_BINARY" "$INSTALL_DIR/$BINARY_NAME"
    chmod +x "$INSTALL_DIR/$BINARY_NAME"

    # 添加到 PATH
    add_to_path "$INSTALL_DIR"

    echo ""
    echo -e "${GREEN}✓ 安装完成!${NC}"
    echo ""
    echo "使用方式:"
    echo "  $BINARY_NAME           # 启动 TUI"
    echo ""
    echo -e "${YELLOW}注意: 请重新加载 shell 配置或重启终端:${NC}"
    case "$shell_type" in
        fish)
            echo "  source ~/.config/fish/config.fish"
            ;;
        zsh)
            echo "  source ~/.zshrc"
            ;;
        bash)
            if [ "$(uname -s)" = "Darwin" ]; then
                echo "  source ~/.bash_profile"
            else
                echo "  source ~/.bashrc"
            fi
            ;;
    esac
    echo ""
    echo -e "${BLUE}配置文件位置:${NC}"
    echo "  macOS: ~/Library/Application Support/pi-capture/config.yaml"
    echo "  Linux: ~/.config/pi-capture/config.yaml"
}

# 卸载功能
uninstall() {
    INSTALL_DIR=$(get_install_dir)
    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        rm "$INSTALL_DIR/$BINARY_NAME"
        echo -e "${GREEN}✓ 已卸载 $BINARY_NAME${NC}"
    else
        echo -e "${YELLOW}$BINARY_NAME 未安装${NC}"
    fi
}

# 帮助信息
show_help() {
    echo "Pi Capture TUI 安装脚本"
    echo ""
    echo "用法:"
    echo "  ./install.sh           安装 pi-capture"
    echo "  ./install.sh uninstall 卸载 pi-capture"
    echo "  ./install.sh help      显示帮助"
}

# 解析参数
case "${1:-}" in
    uninstall)
        uninstall
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        main
        ;;
esac
