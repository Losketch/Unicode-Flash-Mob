
Set-Location -Path $PSScriptRoot

Start-Process -FilePath 'index.html'

Write-Host '进入交互模式…
- 连续输入多行命令（按空行执行）
- 在空缓冲区输入 exit 退出' -ForegroundColor Cyan

function Invoke-REPL {
    $buffer = @()
    while ($true) {
        $prompt = if ($buffer.Count -eq 0) { 'PS> ' } else { '… ' }
        $line = Read-Host -Prompt $prompt

        if ($line -eq 'exit' -and $buffer.Count -eq 0) {
            break
        }

        if ($line -eq '') {
            if ($buffer.Count -gt 0) {
                try {
                    Invoke-Expression ($buffer -join "`n")
                } catch {
                    Write-Host "执行出错： $_" -ForegroundColor Red
                }
                $buffer = @()
            }
        }
        else {
            $buffer += $line
        }
    }
}

Invoke-REPL

Write-Host '已退出交互模式。' -ForegroundColor Green