#requires -Version 5.1
<#
.SYNOPSIS
    StreamInd 一键打包脚本。

.DESCRIPTION
    依次构建：
      1. web_render  —— 远程渲染组件（Vue，Vite lib 模式）
      2. web_ui      —— 控制台（Vue，Vite SPA）
      3. Rust        —— release 版 streamind.exe
    然后按运行期目录约定组装到 dist/streamind-<version>-windows-<arch>/，
    并（默认）压缩成同名 zip。

    产物目录结构：
        streamind.exe
        start.ps1              # 首次运行生成 .env 的启动脚本
        .env.example
        README.md
        prompts/  reader/  sql/
        public/
            dom_view.js  fingerprint_cheat.js
            components/<Name>/index.js   # 由 web_render 产出
            web/                         # 由 web_ui 产出，后端在 / 上托管

.EXAMPLE
    pwsh -File scripts/package.ps1

.EXAMPLE
    # 只重打 Rust 部分（前端产物已就绪）
    pwsh -File scripts/package.ps1 -SkipWebUI -SkipRender

.EXAMPLE
    # 跳过依赖安装（CI 里已装过）
    pwsh -File scripts/package.ps1 -SkipInstall
#>
[CmdletBinding()]
param(
    # 产物输出目录（相对仓库根）
    [string]$OutDir = 'dist',
    # 跳过 web_ui 构建
    [switch]$SkipWebUI,
    # 跳过 web_render 构建
    [switch]$SkipRender,
    # 跳过 Rust release 构建（复用 target/release/streamind.exe）
    [switch]$SkipRust,
    # 缺少 node_modules 时不要自动 pnpm install
    [switch]$SkipInstall,
    # 不生成 zip
    [switch]$NoZip
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Write-Step {
    param([string]$Message)
    Write-Host ''
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Stop-Build {
    param([string]$Message)
    Write-Host "!! $Message" -ForegroundColor Red
    exit 1
}

function Invoke-Pnpm {
    param([string]$ProjectDir, [string[]]$Arguments)
    & pnpm -C $ProjectDir @Arguments
    if ($LASTEXITCODE -ne 0) { Stop-Build "pnpm 执行失败：pnpm -C $ProjectDir $($Arguments -join ' ')" }
}

function Initialize-NodeProject {
    param([string]$ProjectDir)
    if (Test-Path (Join-Path $ProjectDir 'node_modules')) { return }
    if ($SkipInstall) { Stop-Build "$ProjectDir 缺少 node_modules，且指定了 -SkipInstall" }
    Write-Step "安装依赖：$ProjectDir"
    Invoke-Pnpm $ProjectDir @('install')
}

function Copy-Content {
    param([string]$From, [string]$To)
    New-Item -ItemType Directory -Force -Path $To | Out-Null
    Copy-Item -Path (Join-Path $From '*') -Destination $To -Recurse -Force
}

# ---------- 版本与平台 ----------
$versionMatch = Select-String -Path (Join-Path $Root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' |
    Select-Object -First 1
if (-not $versionMatch) { Stop-Build '无法从 Cargo.toml 读取版本号' }
$Version = $versionMatch.Matches[0].Groups[1].Value

$Arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64' } else { 'x64' }
$PackageName = "streamind-$Version-windows-$Arch"
$OutRoot = Join-Path $Root $OutDir
$StageDir = Join-Path $OutRoot $PackageName

Write-Host 'StreamInd 打包' -ForegroundColor Green
Write-Host "  版本 : $Version"
Write-Host "  平台 : windows-$Arch"
Write-Host "  产物 : $StageDir"

# ---------- 1. 渲染组件 ----------
if ($SkipRender) {
    Write-Step '跳过 web_render 构建'
} else {
    Write-Step '构建渲染组件 (web_render)'
    Initialize-NodeProject 'web_render'
    Invoke-Pnpm 'web_render' @('run', 'build-render', '--', '--all')
}

# ---------- 2. 控制台 ----------
if ($SkipWebUI) {
    Write-Step '跳过 web_ui 构建'
} else {
    Write-Step '构建控制台 (web_ui)'
    Initialize-NodeProject 'web_ui'
    # 用 build-only 而不是 build：原 build 脚本依赖未声明的 npm-run-all
    Invoke-Pnpm 'web_ui' @('run', 'build-only')
}

# ---------- 3. Rust ----------
if ($SkipRust) {
    Write-Step '跳过 Rust release 构建'
} else {
    Write-Step '构建 Rust release 二进制'
    & cargo build --release
    if ($LASTEXITCODE -ne 0) { Stop-Build 'cargo build --release 失败' }
}

$Exe = Join-Path $Root 'target/release/streamind.exe'
if (-not (Test-Path $Exe)) { Stop-Build "找不到可执行文件：$Exe" }

# ---------- 4. 组装 ----------
Write-Step "组装到 $StageDir"
if (Test-Path $StageDir) { Remove-Item $StageDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

Copy-Item $Exe $StageDir
Copy-Item (Join-Path $Root 'README.md') $StageDir
Copy-Item (Join-Path $Root '.env.example') $StageDir

foreach ($dir in @('prompts', 'reader', 'sql')) {
    $src = Join-Path $Root $dir
    if (Test-Path $src) { Copy-Content $src (Join-Path $StageDir $dir) }
    else { Write-Warning "缺少目录 $dir" }
}

$PublicStage = Join-Path $StageDir 'public'
New-Item -ItemType Directory -Force -Path $PublicStage | Out-Null

# 浏览器注入脚本：被 .gitignore 忽略，只能从当前工作区带过去
foreach ($js in @('dom_view.js', 'fingerprint_cheat.js')) {
    $src = Join-Path $Root "public/$js"
    if (Test-Path $src) { Copy-Item $src $PublicStage }
    else { Write-Warning "缺少 public/$js，浏览器工具会受影响" }
}

# 渲染组件：优先用 web_render 的构建产物，没有才退回 public/components
$ComponentsSrc = Join-Path $Root 'web_render/dist'
if (-not (Test-Path $ComponentsSrc)) { $ComponentsSrc = Join-Path $Root 'public/components' }
if (Test-Path $ComponentsSrc) {
    Copy-Content $ComponentsSrc (Join-Path $PublicStage 'components')
} else {
    Write-Warning "找不到渲染组件产物：$ComponentsSrc"
}

# 控制台
$WebSrc = Join-Path $Root 'web_ui/dist'
if (Test-Path $WebSrc) {
    Copy-Content $WebSrc (Join-Path $PublicStage 'web')
} else {
    Write-Warning "找不到控制台产物：$WebSrc（包内将没有界面）"
}

# 启动脚本
$StartScript = @'
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

if (-not (Test-Path '.env')) {
    Copy-Item '.env.example' '.env'
    Write-Host '已生成 .env，请先填写 JWT_SECRET_KEY 与 EMBEDDING_API_KEY 后重新运行 start.ps1' -ForegroundColor Yellow
    exit 1
}

Write-Host '启动 StreamInd，随后访问 http://localhost:3000 （登录码见下方日志）' -ForegroundColor Cyan
& "$PSScriptRoot/streamind.exe"
'@
Set-Content -Path (Join-Path $StageDir 'start.ps1') -Value $StartScript -Encoding UTF8

# ---------- 5. 压缩 ----------
$ZipPath = Join-Path $OutRoot "$PackageName.zip"
if (-not $NoZip) {
    Write-Step '压缩产物'
    if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
    Compress-Archive -Path (Join-Path $StageDir '*') -DestinationPath $ZipPath -CompressionLevel Optimal
}

# ---------- 6. 汇总 ----------
$files = Get-ChildItem $StageDir -Recurse -File
$sizeMb = [math]::Round(($files | Measure-Object -Property Length -Sum).Sum / 1MB, 1)
Write-Host ''
Write-Host '打包完成' -ForegroundColor Green
Write-Host "  目录 : $StageDir  ($($files.Count) 个文件, $sizeMb MB)"
if (-not $NoZip) { Write-Host "  压缩 : $ZipPath" }
Write-Host ''
Write-Host '分发与运行：'
Write-Host '  1. 解压后进入目录'
Write-Host '  2. 执行 .\start.ps1（首次会自动生成 .env，填好密钥后再跑一次）'
Write-Host '  3. 浏览器打开 http://localhost:3000'
