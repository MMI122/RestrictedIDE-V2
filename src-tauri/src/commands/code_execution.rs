use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::AppState;

// ─── Event payloads ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct CodeOutput {
    #[serde(rename = "type")]
    pub output_type: String, // "stdout" | "stderr" | "info"
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CodeExit {
    pub code: Option<i32>,
    pub signal: Option<String>,
    pub error: Option<String>,
}

const MAX_EXEC_SECS: u64 = 120;

// ─── Helpers ────────────────────────────────────────────────────────────────

fn emit_output(app: &AppHandle, kind: &str, text: &str) {
    log::debug!("[CODE] emit_output kind={} text={}", kind, text.trim());
    if let Err(e) = app.emit(
        "code-output",
        CodeOutput {
            output_type: kind.into(),
            text: text.into(),
        },
    ) {
        log::error!("[CODE] Failed to emit code-output: {}", e);
    }
}

fn minimal_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    for key in ["PATH", "PATHEXT", "SystemRoot", "TEMP", "TMP", "HOME", "USERPROFILE"] {
        if let Ok(v) = std::env::var(key) {
            env.insert(key.into(), v);
        }
    }
    env
}

fn kill_pid(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F", "/T"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
}

// ─── run_code ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn run_code(
    file_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    log::info!("[CODE] run_code called for: {}", file_path);

    // 1  Kill existing process
    {
        let mut runner = state.running_process.lock().map_err(|e| e.to_string())?;
        if let Some(proc) = runner.take() {
            kill_pid(proc.pid);
        }
    }

    // 2  Validate file access
    {
        let engine = state.policy_engine.lock().map_err(|e| e.to_string())?;
        let r = engine.validate_file_access(&file_path, "read");
        if !r.allowed {
            return Err(format!("Access denied: {}", r.reason.unwrap_or_default()));
        }
    }

    if !Path::new(&file_path).exists() {
        return Err("File not found".into());
    }

    // 3  Determine language
    let ext = Path::new(&file_path)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
        .unwrap_or_default();

    let work_dir = Path::new(&file_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".into());

    let base_name = Path::new(&file_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let file_name = Path::new(&file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let lang_name = match ext.as_str() {
        ".py" => "Python",
        ".js" => "JavaScript (Node.js)",
        ".c" => "C",
        ".cpp" => "C++",
        ".java" => "Java",
        _ => return Err(format!("Unsupported file type: {}", ext)),
    };

    emit_output(&app, "info", &format!("▶ Running {}: {}\n", lang_name, file_name));

    let env = minimal_env();

    // 4  Compiled languages – compile first
    if matches!(ext.as_str(), ".c" | ".cpp" | ".java") {
        let (compiler, compile_args, run_cmd, run_args) = match ext.as_str() {
            ".c" => {
                let out = format!("{}\\{}.exe", work_dir, base_name);
                ("gcc", vec![file_path.clone(), "-o".into(), out.clone()], out, vec![])
            }
            ".cpp" => {
                let out = format!("{}\\{}.exe", work_dir, base_name);
                ("g++", vec![file_path.clone(), "-o".into(), out.clone()], out, vec![])
            }
            ".java" => (
                "javac",
                vec![file_path.clone()],
                "java".into(),
                vec!["-cp".into(), work_dir.clone(), base_name.clone()],
            ),
            _ => unreachable!(),
        };

        emit_output(&app, "info", "⏳ Compiling…\n");

        let compile = Command::new(compiler)
            .args(&compile_args)
            .current_dir(&work_dir)
            .envs(&env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        match compile {
            Ok(out) => {
                if !out.stderr.is_empty() {
                    emit_output(&app, "stderr", &String::from_utf8_lossy(&out.stderr));
                }
                if !out.status.success() {
                    let _ = app.emit(
                        "code-exit",
                        CodeExit { code: out.status.code(), signal: None, error: Some("Compilation failed".into()) },
                    );
                    return Ok(serde_json::json!({ "running": false, "error": "Compilation failed" }));
                }
                emit_output(&app, "info", "✅ Compiled. Running…\n");
            }
            Err(e) => {
                emit_output(&app, "stderr", &format!("Compiler not found: {}. Install it first.\n", compiler));
                let _ = app.emit("code-exit", CodeExit { code: Some(1), signal: None, error: Some(e.to_string()) });
                return Ok(serde_json::json!({ "running": false, "error": e.to_string() }));
            }
        }

        // Spawn executable
        log::info!("[CODE] Compiled successfully, spawning: {}", run_cmd);
        spawn_and_stream(&run_cmd, &run_args, &work_dir, &env, app, state)?;
    } else {
        // 5  Interpreted languages
        let (cmd, args): (&str, Vec<String>) = match ext.as_str() {
            ".py" => ("python", vec![file_path.clone()]),
            ".js" => ("node", vec![file_path.clone()]),
            _ => unreachable!(),
        };
        log::info!("[CODE] Spawning interpreted: {} {:?}", cmd, args);
        spawn_and_stream(cmd, &args, &work_dir, &env, app, state)?;
    }

    log::info!("[CODE] run_code returning success");
    Ok(serde_json::json!({ "running": true, "language": lang_name }))
}

/// Spawn a child process, store it in state, and stream stdout/stderr via events.
fn spawn_and_stream(
    cmd: &str,
    args: &[String],
    work_dir: &str,
    env: &HashMap<String, String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("[CODE] spawn_and_stream: cmd={} args={:?} cwd={}", cmd, args, work_dir);

    let mut child = Command::new(cmd)
        .args(args)
        .current_dir(work_dir)
        .envs(env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            log::error!("[CODE] Failed to spawn process: {}", e);
            emit_output(&app, "stderr", &format!("Failed to start '{}': {}\nMake sure the compiler/interpreter is installed and in your PATH.\n", cmd, e));
            let _ = app.emit("code-exit", CodeExit { code: Some(1), signal: None, error: Some(e.to_string()) });
            e.to_string()
        })?;

    let pid = child.id();
    log::info!("[CODE] Process spawned with PID: {}", pid);
    let stdin = child.stdin.take();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    // Store in state
    {
        let mut runner = state.running_process.lock().map_err(|e| e.to_string())?;
        *runner = Some(crate::RunningProcess { pid, stdin });
    }

    // --- stdout reader ---
    let app_out = app.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().flatten() {
            log::debug!("[CODE] stdout: {}", line);
            if let Err(e) = app_out.emit("code-output", CodeOutput { output_type: "stdout".into(), text: format!("{}\n", line) }) {
                log::error!("[CODE] Failed to emit stdout: {}", e);
            }
        }
        log::debug!("[CODE] stdout stream ended");
    });

    // --- stderr reader ---
    let app_err = app.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().flatten() {
            log::debug!("[CODE] stderr: {}", line);
            if let Err(e) = app_err.emit("code-output", CodeOutput { output_type: "stderr".into(), text: format!("{}\n", line) }) {
                log::error!("[CODE] Failed to emit stderr: {}", e);
            }
        }
        log::debug!("[CODE] stderr stream ended");
    });

    // --- wait + timeout ---
    let app_wait = app.clone();
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    let _ = app_wait.emit("code-exit", CodeExit { code: status.code(), signal: None, error: None });
                    return;
                }
                Ok(None) => {
                    if start.elapsed().as_secs() > MAX_EXEC_SECS {
                        let _ = child.kill();
                        emit_output(&app_wait, "stderr", &format!("\n⏱ Process killed: exceeded {}s time limit.\n", MAX_EXEC_SECS));
                        let _ = app_wait.emit("code-exit", CodeExit { code: None, signal: Some("SIGKILL".into()), error: Some("Timeout".into()) });
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => {
                    let _ = app_wait.emit("code-exit", CodeExit { code: None, signal: None, error: Some(e.to_string()) });
                    return;
                }
            }
        }
    });

    Ok(())
}

// ─── stop_code ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn stop_code(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mut runner = state.running_process.lock().map_err(|e| e.to_string())?;
    if let Some(proc) = runner.take() {
        kill_pid(proc.pid);
        log::info!("[CODE] Process {} stopped", proc.pid);
        Ok(serde_json::json!({ "stopped": true }))
    } else {
        Ok(serde_json::json!({ "stopped": false, "reason": "No running process" }))
    }
}

// ─── send_code_input ────────────────────────────────────────────────────────

#[tauri::command]
pub fn send_code_input(text: String, state: State<'_, AppState>) -> Result<(), String> {
    log::info!("[CODE] send_code_input called with: {:?}", text);
    let mut runner = state.running_process.lock().map_err(|e| e.to_string())?;
    if let Some(ref mut proc) = *runner {
        if let Some(ref mut stdin) = proc.stdin {
            stdin
                .write_all(format!("{}\n", text).as_bytes())
                .map_err(|e| {
                    log::error!("[CODE] stdin write error: {}", e);
                    e.to_string()
                })?;
            stdin.flush().map_err(|e| {
                log::error!("[CODE] stdin flush error: {}", e);
                e.to_string()
            })?;
            log::info!("[CODE] stdin sent successfully");
            Ok(())
        } else {
            log::warn!("[CODE] stdin not available");
            Err("Process stdin not available".into())
        }
    } else {
        log::warn!("[CODE] No running process for stdin");
        Err("No running process".into())
    }
}
