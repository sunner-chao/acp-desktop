@echo off
chcp 65001 >nul
echo ========================================
echo   ACP CC Agent Session Launcher
echo ========================================
echo.

if "%ACP_CLAUDE_PROJECT_DIR%"=="" (
    set "CLAUDE_DIR=%~dp0..\..\claude-code-main"
) else (
    set "CLAUDE_DIR=%ACP_CLAUDE_PROJECT_DIR%"
)

echo [1/3] 启动 code-reviewer 会话...
start "ACP-code-reviewer" cmd /c "cd /d %CLAUDE_DIR% && bun --env-file=.env ./src/entrypoints/cli.tsx -p \"You are 'code-reviewer' - a code review expert. Your address is agent://local/code-reviewer. Review code for security vulnerabilities, bugs, and quality issues. Respond in ACP message format.\""

echo [2/3] 启动 game-strategist 会话...
start "ACP-game-strategist" cmd /c "cd /d %CLAUDE_DIR% && bun --env-file=.env ./src/entrypoints/cli.tsx -p \"You are 'game-strategist' - a game strategy assistant. Your address is agent://local/game-strategist. Specialize in RPG and strategy game guides. Respond in ACP message format.\""

echo [3/3] 启动 security-auditor 会话...
start "ACP-security-auditor" cmd /c "cd /d %CLAUDE_DIR% && bun --env-file=.env ./src/entrypoints/cli.tsx -p \"You are 'security-auditor' - a security audit expert. Your address is agent://local/security-auditor. Detect security vulnerabilities and compliance risks. Respond in ACP message format.\""

echo.
echo ========================================
echo   已启动 3 个 Claude CC 会话
echo ========================================
echo.
echo code-reviewer     - 代码审查专家
echo game-strategist   - 游戏攻略助手
echo security-auditor  - 安全审计专家
echo.
echo 按任意键关闭此窗口...
pause >nul
