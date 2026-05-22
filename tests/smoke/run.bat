@echo off
REM OmniAGP M7 Smoke Test Runner (Windows)
REM Prerequisites: Rust toolchain, LLM service (Ollama/vLLM), optionally Godot 4

setlocal enabledelayedexpansion

set REPO_ROOT=%~dp0..\..
set TIMESTAMP=%date:~0,4%%date:~5,2%%date:~8,2%-%time:~0,2%%time:~3,2%%time:~6,2%
set TIMESTAMP=%TIMESTAMP: =0%

if "%SMOKE_OUTPUT_DIR%"=="" set SMOKE_OUTPUT_DIR=%REPO_ROOT%\output\smoke-%TIMESTAMP%

echo === OmniAGP M7 Smoke Test ===
echo Output: %SMOKE_OUTPUT_DIR%
echo.

REM Check prerequisites
echo [1/5] Checking prerequisites...
where cargo >nul 2>&1 || (echo ERROR: cargo not found & exit /b 1)

if "%LLM_BASE_URL%"=="" (
    set LLM_BASE_URL=http://localhost:11434/v1
    echo   LLM_BASE_URL defaulting to http://localhost:11434/v1
)

if "%LLM_MODEL%"=="" (
    set LLM_MODEL=qwen2.5-coder-7b
    echo   LLM_MODEL defaulting to qwen2.5-coder-7b
)

where godot >nul 2>&1 && (
    echo   Godot: found
) || (
    echo   Godot: not found ^(headless QA and export will use stubs^)
)

echo.

REM Build
echo [2/5] Building smoke test binary...
cd /d %REPO_ROOT%
cargo build --release -p omni-smoke-test
if errorlevel 1 (
    echo ERROR: Build failed
    exit /b 1
)
echo   Build complete.
echo.

REM Run
echo [3/5] Running end-to-end smoke test...
set RUST_LOG=info
cargo run --release -p omni-smoke-test
set EXIT_CODE=%errorlevel%

echo.
echo [4/5] Results:
echo   Exit code: %EXIT_CODE%

if exist "%SMOKE_OUTPUT_DIR%\smoke-test-report.json" (
    echo   Report: %SMOKE_OUTPUT_DIR%\smoke-test-report.json
)

echo.
echo [5/5] Artifacts:
if exist "%SMOKE_OUTPUT_DIR%" dir /b "%SMOKE_OUTPUT_DIR%"

if %EXIT_CODE% equ 0 (
    echo.
    echo === SMOKE TEST PASSED ===
) else (
    echo.
    echo === SMOKE TEST FAILED ===
)

exit /b %EXIT_CODE%
