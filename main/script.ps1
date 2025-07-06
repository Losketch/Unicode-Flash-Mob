
[CmdletBinding()]
param()

Set-Location -Path $PSScriptRoot

$html = Join-Path $PSScriptRoot 'index.html'
if (Test-Path $html) {
    Start-Process -FilePath $html
} else {
    Write-Error "找不到文件：$html"
}

Write-Host '
进入交互模式：
  - 输入 PowerShell 命令，空行后执行
  - 直接输入 exit 退出
' -ForegroundColor Cyan

function Invoke-REPL {
    $buffer = @()
    while ($true) {
        $prompt = if ($buffer.Count -eq 0) { 'PS> ' } else { '... ' }
        try {
            $line = Read-Host -Prompt $prompt
        }
        catch {
            break
        }

        if ($line -eq 'exit' -and $buffer.Count -eq 0) { break }

        if ([string]::IsNullOrWhiteSpace($line)) {
            if ($buffer.Count -gt 0) {
                $scriptBlock = [ScriptBlock]::Create($buffer -join "`n")
                try {
                    & $scriptBlock
                }
                catch {
                    Write-Host "执行出错： $_" -ForegroundColor Red
                }
                $buffer.Clear()
            }
        }
        else {
            $buffer += $line
        }
    }
}

Invoke-REPL

Write-Host '已退出交互模式。' -ForegroundColor Green