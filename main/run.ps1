
Set-Location -Path $PSScriptRoot

function Confirm-Proceed {
    param (
        [string]$Message = "是否继续？ (y/n)"
    )
    $response = Read-Host $Message
    if ($response -eq 'y' -or $response -eq 'Y') {
        return $true
    } else {
        Write-Host "操作已取消。" -ForegroundColor Yellow
        return $false
    }
}


Write-Host "--- 步骤 1: 尝试更新数据 ---" -ForegroundColor Cyan
if (Confirm-Proceed "是否开始更新数据？ (y/n)") {
    Write-Host "正在执行 download..." -ForegroundColor Green
    ./unicode_flash_mob.exe download
    if ($LASTEXITCODE -ne 0) {
        Write-Host "download 执行失败，请检查错误。" -ForegroundColor Red
        exit 1
    }
    Write-Host "数据更新完成。" -ForegroundColor Green
} else {
    Write-Host "已跳过数据更新步骤。" -ForegroundColor Yellow
}
Write-Host ""


Write-Host "--- 步骤 2: 重置配置设置并生成 DecipherUnicode*.txt 文件 ---" -ForegroundColor Cyan
if (Confirm-Proceed "是否 重置配置设置 并生成 DecipherUnicode*.txt 文件？ (y/n)") {
    Write-Host "正在执行 write-settings..." -ForegroundColor Green
    ./unicode_flash_mob.exe write-settings
    if ($LASTEXITCODE -ne 0) {
        Write-Host "write-settings 执行失败，请检查错误。" -ForegroundColor Red
        exit 1
    }

    if (Confirm-Proceed "是否执行 process-unicode-block (可选)？ (y/n)") {
        Write-Host "正在执行 process-unicode-block..." -ForegroundColor Green
        ./unicode_flash_mob.exe process-unicode-block
        if ($LASTEXITCODE -ne 0) {
            Write-Host "process-unicode-block 执行失败，请检查错误。" -ForegroundColor Red
        }
    } else {
        Write-Host "已跳过 process-unicode-block。" -ForegroundColor Yellow
    }

    if (Confirm-Proceed "是否执行 process-unicode-data (可选)？ (y/n)") {
        Write-Host "正在执行 process-unicode-data..." -ForegroundColor Green
        ./unicode_flash_mob.exe process-unicode-data
        if ($LASTEXITCODE -ne 0) {
            Write-Host "process-unicode-data 执行失败，请检查错误。" -ForegroundColor Red
        }
    } else {
        Write-Host "已跳过 process-unicode-data。" -ForegroundColor Yellow
    }
    Write-Host "DecipherUnicode*.txt 文件生成及配置设置完成。" -ForegroundColor Green
} else {
    Write-Host "已跳过 DecipherUnicode*.txt 文件生成及配置设置步骤。" -ForegroundColor Yellow
}
Write-Host ""


Write-Host "--- 步骤 3: 生成测试文档 ---" -ForegroundColor Cyan
if (Confirm-Proceed "是否生成测试文档？ (y/n)") {
    $useFontGenerator = Confirm-Proceed "是否从字体文件生成测试文档？ (y/n)"

    if ($useFontGenerator) {
        Write-Host "从字体生成测试文档"
        $fontFiles = Read-Host "请输入字体文件路径 (多个文件用空格分隔，例如: 'font1.ttf font2.otf')，或留空以跳过此步骤："
        if ([string]::IsNullOrWhiteSpace($fontFiles)) {
            Write-Host "未输入字体文件，已跳过字体测试文档生成步骤。" -ForegroundColor Yellow
        } else {
            Write-Host "正在执行 extract..." -ForegroundColor Green
            $fontFilesArray = $fontFiles.Split(' ', [System.StringSplitOptions]::RemoveEmptyEntries)
            ./unicode_flash_mob.exe extract $fontFilesArray
            if ($LASTEXITCODE -ne 0) {
                Write-Host "extract 执行失败，请检查错误。" -ForegroundColor Red
                exit 1
            }
            Write-Host "字体测试文档生成完成。" -ForegroundColor Green
        }
    } else {
        $useRangeGenerator = Confirm-Proceed "是否从 16 进制生成测试文档？ (y/n)"

        if ($useRangeGenerator) {
            $startHex = Read-Host "请输入起始 Unicode 范围 (十六进制，例如：0000)"
            $endHex = Read-Host "请输入结束 Unicode 范围 (十六进制，例如：FFFF)"
            $fontPath = Read-Host "请输入字体文件路径 (例如：arial.ttf，仅限单一字体)"

            if ([string]::IsNullOrWhiteSpace($fontPath)) {
                Write-Host "未输入字体文件路径，已跳过此步骤。" -ForegroundColor Yellow
            } else {
                Write-Host "正在执行 generate-unicode-range..." -ForegroundColor Green
                ./unicode_flash_mob.exe generate-unicode-range --file combined_unicode_list.txt --start $startHex --end $endHex --font $fontPath
                if ($LASTEXITCODE -ne 0) {
                    Write-Host "generate-unicode-range 执行失败，请检查错误。" -ForegroundColor Red
                    exit 1
                }
                Write-Host "字体测试文档生成完成。" -ForegroundColor Green
            }
        } else {
            Write-Host "已跳过从 16 进制范围生成测试文档步骤。" -ForegroundColor Yellow
        }
    }
} else {
    Write-Host "已跳过从字体生成测试文档步骤。" -ForegroundColor Yellow
}
Write-Host ""


Write-Host "--- 步骤 4: 重命名 combined_unicode_list.txt 为 Unicode.txt ---" -ForegroundColor Cyan
if (Confirm-Proceed "是否重命名 'combined_unicode_list.txt' 为 'Unicode.txt'？ (y/n)") {
    Write-Host "等待 100 毫秒..." -ForegroundColor DarkGray
    Start-Sleep -m 100

    if (Test-Path 'combined_unicode_list.txt') {
        Write-Host "正在重命名文件..." -ForegroundColor Green
        Rename-Item 'combined_unicode_list.txt' -NewName 'Unicode.txt' -ErrorAction Stop
        Write-Host "文件重命名完成。" -ForegroundColor Green
    } else {
        Write-Host "文件 'combined_unicode_list.txt' 不存在，跳过重命名。" -ForegroundColor Yellow
    }
} else {
    Write-Host "已跳过文件重命名步骤。" -ForegroundColor Yellow
}
Write-Host ""


Write-Host "--- 步骤 5: 执行 集成字体 Unicode 提取与说明文件替换工具 ---" -ForegroundColor Cyan
if (Confirm-Proceed "是否执行 集成字体 Unicode 提取与说明文件替换工具？ (y/n)") {
    Write-Host "正在执行 replace-unicode-data 1..." -ForegroundColor Green
    ./unicode_flash_mob.exe replace-unicode-data 1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "replace-unicode-data 执行失败，请检查错误。" -ForegroundColor Red
        exit 1
    }
    Write-Host "replace-unicode-data 完成。" -ForegroundColor Green
} else {
    Write-Host "已跳过 replace-unicode-data 步骤。" -ForegroundColor Yellow
}
Write-Host ""


Write-Host "--- 步骤 6: Python 部分 - 生成 PNG 文件 ---" -ForegroundColor Cyan
if (Confirm-Proceed "是否生成 PNG 文件？ (y/n)") {
    Write-Host "请选择 PNG 生成模式：" -ForegroundColor White
    Write-Host "1. 背景变化 (生成png（背景变化）.py)" -ForegroundColor White
    Write-Host "2. 背景不变 (生成png（背景不变）.py)" -ForegroundColor White
    $choice = Read-Host "请输入您的选择 (1/2)，或输入 'n' 跳过此步骤："

    $scriptToRun = ""
    if ($choice -eq '1') {
        $scriptToRun = "生成png（背景变化）.py"
    } elseif ($choice -eq '2') {
        if (Test-Path ".\生成png（背景不变）.py") {
            $scriptToRun = "生成png（背景不变）.py"
        } else {
            Write-Host "文件 '.\生成png（背景不变）.py' 不存在，请检查。" -ForegroundColor Red
            $scriptToRun = ""
        }
    } else {
        Write-Host "无效的选择或已跳过 PNG 生成步骤。" -ForegroundColor Yellow
    }

    if (-not [string]::IsNullOrWhiteSpace($scriptToRun)) {
        Write-Host "正在执行 .\python\python.exe .\$scriptToRun..." -ForegroundColor Green
        & ".\python\python.exe" ".\$scriptToRun"
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Python 脚本 '$scriptToRun' 执行失败，请检查错误。" -ForegroundColor Red
            exit 1
        }
        Write-Host "PNG 文件生成完成。" -ForegroundColor Green
    }
} else {
    Write-Host "已跳过 PNG 文件生成步骤。" -ForegroundColor Yellow
}
Write-Host ""


Write-Host "--- 步骤 7: Python 部分 - 生成视频 (MP4) ---" -ForegroundColor Cyan
if (Confirm-Proceed "是否生成 MP4 视频文件？ (y/n)") {
    Write-Host "正在执行 .\python\python.exe .\生成mp4文件.py..." -ForegroundColor Green
    & ".\python\python.exe" ".\生成mp4文件.py"
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Python 脚本 '生成mp4文件.py' 执行失败，请检查错误。" -ForegroundColor Red
        exit 1
    }
    Write-Host "MP4 视频文件生成完成。" -ForegroundColor Green
} else {
    Write-Host "已跳过 MP4 视频文件生成步骤。" -ForegroundColor Yellow
}
Write-Host ""


Write-Host "--- 步骤 8: 打开 output 目录 ---" -ForegroundColor Cyan
if (Confirm-Proceed "是否打开输出目录？ (y/n)") {
    $outputDir = ".\output"

    if (Test-Path $outputDir -PathType Container) {
        Write-Host "正在打开目录 '$outputDir'..." -ForegroundColor Green
        Invoke-Item $outputDir
        Write-Host "目录已打开。" -ForegroundColor Green
    } else {
        Write-Host "目录 '$outputDir' 不存在，无法打开。" -ForegroundColor Red
    }
} else {
    Write-Host "已跳过打开输出目录步骤。" -ForegroundColor Yellow
}
Write-Host ""

Write-Host "所有可选步骤已完成或跳过。" -ForegroundColor Green