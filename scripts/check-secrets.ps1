# 提交前密钥自检：扫描暂存区（或指定路径）中的常见密钥特征
# 用法：
#   pwsh -File scripts/check-secrets.ps1            # 扫描暂存区
#   pwsh -File scripts/check-secrets.ps1 -All       # 扫描工作区全部受跟踪文件
# 可挂到 .git/hooks/pre-commit：
#   pwsh -File scripts/check-secrets.ps1; if ($LASTEXITCODE -ne 0) { exit 1 }
param(
    [switch]$All
)

$ErrorActionPreference = 'Stop'

# 明显的密钥特征；如需调整请谨慎，避免把正常代码误判
$patterns = @(
    'sk-[A-Za-z0-9]{20,}',
    'ark-[0-9a-f-]{20,}',
    'AKIA[0-9A-Z]{16}',
    'gh[pousr]_[A-Za-z0-9]{20,}',
    '-----BEGIN (RSA|OPENSSH|EC) PRIVATE KEY-----'
)

$files = if ($All) {
    git ls-files
} else {
    git diff --cached --name-only --diff-filter=ACM
}

if (-not $files) {
    Write-Host '✅ 没有需要检查的文件'
    exit 0
}

$hits = 0
foreach ($file in $files) {
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { continue }
    # 跳过明显的大文件 / 二进制
    $item = Get-Item -LiteralPath $file
    if ($item.Length -gt 2MB) { continue }

    $lineNo = 0
    foreach ($line in Get-Content -LiteralPath $file -ErrorAction SilentlyContinue) {
        $lineNo++
        foreach ($p in $patterns) {
            if ($line -match $p) {
                Write-Host "❌ ${file}:$lineNo  命中密钥特征 /$p/"
                $hits++
                break
            }
        }
    }
}

if ($hits -gt 0) {
    Write-Host ""
    Write-Host "发现 $hits 处疑似密钥，请先移除或改用环境变量（见 README「安全须知」）。" -ForegroundColor Red
    exit 1
}

Write-Host '✅ 未发现疑似密钥'
exit 0
