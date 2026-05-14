# ACP Agent Launcher - 为每个 cc 智能体启动独立 Claude 会话
param(
    [switch]$NoApp  # 不启动 ACP 桌面应用
)

$CLAUDE_DIR = $env:ACP_CLAUDE_PROJECT_DIR
if (-not $CLAUDE_DIR) {
    $CLAUDE_DIR = Join-Path $PSScriptRoot "..\..\claude-code-main"
}
$TEMPLATES = "$PSScriptRoot\..\templates\agents.json"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  ACP Multi-Agent System Launcher" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 读取智能体配置
$agents = Get-Content $TEMPLATES -Raw | ConvertFrom-Json

# 分类智能体
$ccAgents = $agents | Where-Object { $_.config.apiFormat -eq "anthropic" }
$openaiAgents = $agents | Where-Object { $_.config.apiFormat -eq "openai" }
$scriptAgents = $agents | Where-Object { $_.driverType -eq "script" }

Write-Host "[Config] 共 $($agents.Count) 个智能体:" -ForegroundColor White
Write-Host "  CC (Anthropic): $($ccAgents.Count) 个" -ForegroundColor Green
Write-Host "  OpenAI: $($openaiAgents.Count) 个" -ForegroundColor Yellow
Write-Host "  Script: $($scriptAgents.Count) 个" -ForegroundColor Magenta
Write-Host ""

# 为每个 cc 智能体启动独立 Claude 会话
$sessions = @()
foreach ($agent in $ccAgents) {
    $sessionName = $agent.name
    $roleDesc = $agent.description
    $model = $agent.config.model

    # 构建 Claude 启动命令 (使用 Git Bash)
    $prompt = "You are now acting as the agent '$($agent.name)'. Your role: $($agent.description). Your address: $($agent.address). You are part of an ACP (Agent Communication Protocol) network. Respond to messages concisely and follow ACP message format. When you receive a message from another agent, respond appropriately based on your role."

    # 创建独立会话目录
    $sessionDir = "$env:USERPROFILE\.claude\acp-sessions\$sessionName"
    New-Item -ItemType Directory -Force -Path $sessionDir | Out-Null

    Write-Host "[Launch] 启动智能体: $sessionName" -ForegroundColor Green
    Write-Host "  模型: $model" -ForegroundColor Gray
    Write-Host "  角色: $roleDesc" -ForegroundColor Gray
    Write-Host "  会话目录: $sessionDir" -ForegroundColor Gray

    # 在 Git Bash 中启动 Claude 会话
    $bashPath = "C:\Program Files\Git\bin\bash.exe"

    if (Test-Path $bashPath) {
        $claudeCmd = "cd `"$CLAUDE_DIR`" && CLAUDE_CODE_SESSION_NAME=$sessionName bun --env-file=.env ./src/entrypoints/cli.tsx -p `"$prompt`""

        $proc = Start-Process -FilePath $bashPath `
            -ArgumentList "-c", $claudeCmd `
            -PassThru `
            -WindowStyle Normal

        $sessions += @{
            Agent = $sessionName
            Process = $proc
            StartedAt = Get-Date
        }

        Write-Host "  PID: $($proc.Id) | 状态: 已启动" -ForegroundColor Cyan
    } else {
        Write-Host "  [WARN] Git Bash 未找到，尝试直接使用 bun" -ForegroundColor Yellow

        # 直接在 PowerShell 中启动
        $env:CLAUDE_CODE_SESSION_NAME = $sessionName
        $proc = Start-Process -FilePath "bun" `
            -ArgumentList "--env-file=.env", "./src/entrypoints/cli.tsx", "-p", "`"$prompt`"" `
            -WorkingDirectory $CLAUDE_DIR `
            -PassThru `
            -WindowStyle Normal

        $sessions += @{
            Agent = $sessionName
            Process = $proc
            StartedAt = Get-Date
        }

        Write-Host "  PID: $($proc.Id) | 状态: 已启动" -ForegroundColor Cyan
    }

    Write-Host ""
    Start-Sleep -Seconds 2
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  已启动 $($sessions.Count) 个 Claude 会话" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 显示运行中的会话
Write-Host "运行中的智能体会话:" -ForegroundColor White
foreach ($s in $sessions) {
    Write-Host "  [$($s.Agent)] PID: $($s.Process.Id) | 启动时间: $($s.StartedAt)" -ForegroundColor Gray
}

# 保存会话状态
$sessionsJson = $sessions | Select-Object Agent, @{N='PID';E={$_.Process.Id}}, StartedAt | ConvertTo-Json
$sessionsJson | Out-File -FilePath "$PSScriptRoot\..\sessions.json" -Force

Write-Host ""
Write-Host "会话状态已保存到: sessions.json" -ForegroundColor Gray
Write-Host ""
Write-Host "按 Ctrl+C 停止所有会话" -ForegroundColor Yellow
Write-Host ""

# 启动 ACP 桌面应用
if (-not $NoApp) {
    Write-Host "[Launch] 启动 ACP Desktop Agent Hub..." -ForegroundColor Cyan

    $acpDir = "$PSScriptRoot\.."
    Start-Process -FilePath "npm" `
        -ArgumentList "run", "tauri", "dev" `
        -WorkingDirectory $acpDir `
        -WindowStyle Normal

    Write-Host "ACP Desktop 已启动" -ForegroundColor Green
}

# 保持脚本运行，直到用户终止
try {
    while ($true) {
        Start-Sleep -Seconds 30

        # 检查会话状态
        $alive = 0
        foreach ($s in $sessions) {
            if (-not $s.Process.HasExited) {
                $alive++
            }
        }

        if ($alive -eq 0) {
            Write-Host "[WARN] 所有会话已退出" -ForegroundColor Red
            break
        }
    }
} finally {
    Write-Host ""
    Write-Host "正在清理..." -ForegroundColor Yellow

    foreach ($s in $sessions) {
        if (-not $s.Process.HasExited) {
            Write-Host "  停止: $($s.Agent) (PID: $($s.Process.Id))" -ForegroundColor Gray
            $s.Process.Kill()
        }
    }

    Write-Host "所有会话已停止" -ForegroundColor Green
}
