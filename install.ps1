#!/usr/bin/env pwsh

# 设置安装路径（使用当前目录）
$CURRENT_DIR = Get-Location
$INSTALL_DIR = Join-Path $CURRENT_DIR "kokoros"
$CONFIG_DIR = Join-Path $INSTALL_DIR "config"
$VOICES_JSON_SRC = "data\voices.json"
$VOICES_JSON_DEST = Join-Path $CONFIG_DIR "voices.json"
$KOKO_BIN_SRC = "target\release\koko.exe"
$KOKO_BIN_DEST = Join-Path $INSTALL_DIR "koko.exe"

# 检查并清理全局 Cargo 配置
$USER_CARGO_CONFIG = Join-Path $env:USERPROFILE ".cargo\config.toml"
if (Test-Path $USER_CARGO_CONFIG) {
    Write-Host "发现全局 Cargo 配置，正在备份并移除..."
    Copy-Item $USER_CARGO_CONFIG "$USER_CARGO_CONFIG.bak"
    Remove-Item $USER_CARGO_CONFIG
}

# 创建必要的目录
Write-Host "创建安装目录..."
New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
New-Item -ItemType Directory -Force -Path $CONFIG_DIR | Out-Null

# 检查并复制 voices.json
if (Test-Path $VOICES_JSON_SRC) {
    Write-Host "复制配置文件 $VOICES_JSON_SRC 到 $VOICES_JSON_DEST"
    Copy-Item $VOICES_JSON_SRC $VOICES_JSON_DEST -Force
} else {
    Write-Error "错误: 未找到 $VOICES_JSON_SRC"
    exit 1
}

# 检查是否已安装 Rust
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "未检测到 Rust，请先安装 Rust 环境: https://rustup.rs/"
    exit 1
}

# 编译项目
Write-Host "正在编译项目..."
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "编译失败"
    exit 1
}

# 复制二进制文件
Write-Host "复制 $KOKO_BIN_SRC 到 $KOKO_BIN_DEST"
Copy-Item $KOKO_BIN_SRC $KOKO_BIN_DEST -Force

Write-Host "`n安装完成！"
Write-Host "配置文件位置: $VOICES_JSON_DEST"
Write-Host "程序安装位置: $KOKO_BIN_DEST"
Write-Host "要运行程序，请使用: $KOKO_BIN_DEST"

# 创建启动脚本，使用线程构建器来设置栈大小
$LAUNCH_SCRIPT = Join-Path $INSTALL_DIR "start.ps1"
@"
`$env:KOKOROS_CONFIG = '$CONFIG_DIR'
# 注意：这个环境变量对主线程无效，所以我们需要在代码中使用线程构建器
`$env:RUST_MIN_STACK = "16777216"  # 16MB 栈大小
& '$KOKO_BIN_DEST' `$args
"@ | Out-File -FilePath $LAUNCH_SCRIPT -Encoding UTF8

# 如果存在备份的配置，在安装完成后恢复
if (Test-Path "$USER_CARGO_CONFIG.bak") {
    Write-Host "正在恢复全局 Cargo 配置..."
    Move-Item "$USER_CARGO_CONFIG.bak" $USER_CARGO_CONFIG -Force
}

Write-Host "`n你也可以使用启动脚本运行: $LAUNCH_SCRIPT" 