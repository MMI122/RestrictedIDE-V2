use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use tauri::State;

use crate::commands::session_commands::SessionState;

// ─── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct JudgeResultEntry {
    pub submission_id: String,
    pub student_id: String,
    pub filename: String,
    pub lang: Option<String>,
    pub result: String,       // "pass" | "fail" | "partial" | "compile_error" | "timeout"
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub exec_time_ms: Option<u32>,
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn minimal_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    for key in ["PATH", "PATHEXT", "SystemRoot", "TEMP", "TMP", "HOME", "USERPROFILE"] {
        if let Ok(v) = std::env::var(key) {
            env.insert(key.into(), v);
        }
    }
    env
}

fn safe_filename(filename: &str) -> String {
    Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "submission.txt".to_string())
}

fn judge_result_to_str(result: &str) -> &str {
    match result {
        "pass" => "pass",
        "partial" => "partial",
        "fail" => "fail",
        "compile_error" => "compile_error",
        "timeout" => "timeout",
        _ => "pending",
    }
}

fn judge_result_enum_to_str(result: &crate::session::models::JudgeResult) -> &'static str {
    match result {
        crate::session::models::JudgeResult::Pass => "pass",
        crate::session::models::JudgeResult::Partial => "partial",
        crate::session::models::JudgeResult::Fail => "fail",
        crate::session::models::JudgeResult::CompileError => "compile_error",
        crate::session::models::JudgeResult::Timeout => "timeout",
        crate::session::models::JudgeResult::Pending => "pending",
    }
}

fn judge_one(
    filename: &str,
    content: &str,
    lang: Option<&str>,
    input_data: Option<&str>,
    expected_output: Option<&str>,
    time_limit_ms: u32,
) -> (String, Option<String>, Option<String>, Option<u32>) {
    let safe_name = safe_filename(filename);

    // Determine language from filename extension or lang field
    let ext = Path::new(&safe_name)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let lang_key = lang
        .map(|l| l.to_lowercase())
        .unwrap_or_else(|| ext.clone());

    // Create a temp directory for this judge run
    let tmp_dir = std::env::temp_dir().join(format!("ride_judge_{}", uuid::Uuid::new_v4()));
    if std::fs::create_dir_all(&tmp_dir).is_err() {
        return ("compile_error".into(), None, Some("Failed to create temp dir".into()), None);
    }

    // Write source file
    let src_path = tmp_dir.join(&safe_name);
    if let Some(parent) = src_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&src_path, content).is_err() {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return ("compile_error".into(), None, Some("Failed to write source".into()), None);
    }

    let env = minimal_env();
    let base_name = Path::new(&safe_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    // For compiled languages, compile first
    let run_cmd: String;
    let run_args: Vec<String>;

    match lang_key.as_str() {
        "c" => {
            let out = tmp_dir.join(format!("{}.exe", base_name));
            let compile = Command::new("gcc")
                .args([
                    src_path.to_string_lossy().as_ref(),
                    "-o",
                    out.to_string_lossy().as_ref(),
                ])
                .current_dir(&tmp_dir)
                .envs(&env)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();
            match compile {
                Ok(o) if !o.status.success() => {
                    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                    let _ = std::fs::remove_dir_all(&tmp_dir);
                    return ("compile_error".into(), None, Some(stderr), None);
                }
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&tmp_dir);
                    return ("compile_error".into(), None, Some(format!("gcc not found: {}", e)), None);
                }
                _ => {}
            }
            run_cmd = out.to_string_lossy().to_string();
            run_args = vec![];
        }
        "cpp" | "c++" => {
            let out = tmp_dir.join(format!("{}.exe", base_name));
            let compile = Command::new("g++")
                .args([
                    src_path.to_string_lossy().as_ref(),
                    "-o",
                    out.to_string_lossy().as_ref(),
                ])
                .current_dir(&tmp_dir)
                .envs(&env)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();
            match compile {
                Ok(o) if !o.status.success() => {
                    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                    let _ = std::fs::remove_dir_all(&tmp_dir);
                    return ("compile_error".into(), None, Some(stderr), None);
                }
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&tmp_dir);
                    return ("compile_error".into(), None, Some(format!("g++ not found: {}", e)), None);
                }
                _ => {}
            }
            run_cmd = out.to_string_lossy().to_string();
            run_args = vec![];
        }
        "java" => {
            let compile = Command::new("javac")
                .arg(src_path.to_string_lossy().as_ref())
                .current_dir(&tmp_dir)
                .envs(&env)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output();
            match compile {
                Ok(o) if !o.status.success() => {
                    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                    let _ = std::fs::remove_dir_all(&tmp_dir);
                    return ("compile_error".into(), None, Some(stderr), None);
                }
                Err(e) => {
                    let _ = std::fs::remove_dir_all(&tmp_dir);
                    return ("compile_error".into(), None, Some(format!("javac not found: {}", e)), None);
                }
                _ => {}
            }
            run_cmd = "java".into();
            run_args = vec![
                "-cp".into(),
                tmp_dir.to_string_lossy().to_string(),
                base_name.clone(),
            ];
        }
        "python" | "py" => {
            run_cmd = "python".into();
            run_args = vec![src_path.to_string_lossy().to_string()];
        }
        "javascript" | "js" => {
            run_cmd = "node".into();
            run_args = vec![src_path.to_string_lossy().to_string()];
        }
        _ => {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return (
                "compile_error".into(),
                None,
                Some(format!("Unsupported language: {}", lang_key)),
                None,
            );
        }
    }

    // Run the program with optional stdin
    let timeout = std::time::Duration::from_millis(time_limit_ms.max(1000) as u64);
    let start = std::time::Instant::now();

    let child = Command::new(&run_cmd)
        .args(&run_args)
        .current_dir(&tmp_dir)
        .envs(&env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return (
                "compile_error".into(),
                None,
                Some(format!("Failed to run: {}", e)),
                None,
            );
        }
    };

    // Write stdin if provided
    if let Some(input) = input_data {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
            // Drop stdin to close it and let the program continue
        }
    } else {
        // Drop stdin immediately
        drop(child.stdin.take());
    }

    // Wait with timeout
    let pid = child.id();
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {
                if start.elapsed() > timeout {
                    kill_pid(pid);
                    let _ = child.wait(); // reap
                    break Err("timeout");
                }
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(_) => break Err("error"),
        }
    };

    let elapsed_ms = start.elapsed().as_millis() as u32;

    let (stdout_str, stderr_str) = match &result {
        Ok(_) => {
            let mut stdout_buf = String::new();
            let mut stderr_buf = String::new();
            if let Some(mut out) = child.stdout.take() {
                let _ = out.read_to_string(&mut stdout_buf);
            }
            if let Some(mut err) = child.stderr.take() {
                let _ = err.read_to_string(&mut stderr_buf);
            }
            (stdout_buf, stderr_buf)
        }
        Err(_) => (String::new(), String::new()),
    };

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp_dir);

    match result {
        Err("timeout") => (
            "timeout".into(),
            Some(stdout_str),
            Some("Process killed: exceeded time limit".into()),
            Some(elapsed_ms),
        ),
        Err(_) => (
            "fail".into(),
            Some(stdout_str),
            Some(stderr_str),
            Some(elapsed_ms),
        ),
        Ok(status) if !status.success() => (
            "fail".into(),
            Some(stdout_str),
            Some(if stderr_str.is_empty() {
                format!("Exit code: {:?}", status.code())
            } else {
                stderr_str
            }),
            Some(elapsed_ms),
        ),
        Ok(_) => {
            // Compare output to expected
            let judge = match expected_output {
                Some(expected) => {
                    let actual = stdout_str.trim();
                    let expected = expected.trim();
                    if actual == expected {
                        "pass"
                    } else {
                        // Basic partial scoring: line-level overlap ratio.
                        let actual_lines: Vec<&str> = actual
                            .lines()
                            .map(|l| l.trim())
                            .filter(|l| !l.is_empty())
                            .collect();
                        let expected_lines: Vec<&str> = expected
                            .lines()
                            .map(|l| l.trim())
                            .filter(|l| !l.is_empty())
                            .collect();

                        if !expected_lines.is_empty() {
                            let matched = expected_lines
                                .iter()
                                .filter(|line| actual_lines.contains(line))
                                .count();
                            let ratio = matched as f32 / expected_lines.len() as f32;
                            if ratio >= 0.4 {
                                "partial"
                            } else {
                                "fail"
                            }
                        } else {
                            "fail"
                        }
                    }
                }
                None => "pass", // no expected output → just check it ran OK
            };
            (
                judge.into(),
                Some(stdout_str),
                if stderr_str.is_empty() {
                    None
                } else {
                    Some(stderr_str)
                },
                Some(elapsed_ms),
            )
        }
    }
}

fn kill_pid(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F", "/T"])
            .creation_flags(0x08000000)
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// ─── Commands ───────────────────────────────────────────────────────────────

/// Batch judge all final submissions for a session.
/// For each submission, looks up the corresponding question's input_data and expected_output.
#[tauri::command]
pub async fn judge_submissions_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
) -> Result<Vec<JudgeResultEntry>, String> {
    let db = session_state.db.clone();

    // Get questions (for input/expected output per question)
    let questions = db.get_questions(&session_id).map_err(|e| e.to_string())?;

    // Build a map from question order index (0-based) to question
    // For now, we use the first question's input/expected for all submissions
    // since students submit one file per session currently.
    let default_input = questions.first().and_then(|q| q.input_data.clone());
    let default_expected = questions.first().and_then(|q| q.expected_output.clone());
    let default_time_limit = questions.first().map(|q| q.time_limit_ms).unwrap_or(5000);

    let submissions = db
        .get_final_submissions(&session_id)
        .map_err(|e| e.to_string())?;

    // Run judging in a blocking task (involves process spawning)
    let results = tokio::task::spawn_blocking(move || {
        let mut entries = Vec::new();
        for sub in &submissions {
            let (result, stdout, stderr, exec_ms) = judge_one(
                &sub.filename,
                &sub.content,
                sub.lang.as_deref(),
                default_input.as_deref(),
                default_expected.as_deref(),
                default_time_limit,
            );

            // Update DB
            let _ = db.update_submission_result(
                &sub.id,
                judge_result_to_str(&result),
                stdout.as_deref(),
                stderr.as_deref(),
                exec_ms,
            );

            entries.push(JudgeResultEntry {
                submission_id: sub.id.clone(),
                student_id: sub.student_id.clone(),
                filename: sub.filename.clone(),
                lang: sub.lang.clone(),
                result,
                stdout,
                stderr,
                exec_time_ms: exec_ms,
            });
        }
        entries
    })
    .await
    .map_err(|e| format!("Judge task failed: {}", e))?;

    log::info!(
        "[Judge] Judged {} submissions for session {}",
        results.len(),
        session_id
    );
    Ok(results)
}

/// Download all final submissions as a zip file. Returns the path to the saved zip.
#[tauri::command]
pub async fn download_submissions_zip_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
    save_dir: String,
) -> Result<String, String> {
    let db = session_state.db.clone();

    let session = db
        .get_session_by_id(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or("Session not found")?;

    let submissions = db
        .get_final_submissions(&session_id)
        .map_err(|e| e.to_string())?;

    if submissions.is_empty() {
        return Err("No submissions to download".into());
    }

    let folder_name = format!("session-{}", session.code);
    let zip_filename = format!("{}.zip", folder_name);
    let zip_path = Path::new(&save_dir).join(&zip_filename);

    std::fs::create_dir_all(&save_dir)
        .map_err(|e| format!("Failed to create save directory: {}", e))?;

    // Build the zip in memory then write to disk
    let zip_bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            for sub in &submissions {
                let entry_path = format!(
                    "{}/{}/{}",
                    folder_name,
                    sub.student_id,
                    safe_filename(&sub.filename)
                );
                zip.start_file(entry_path, options)
                    .map_err(|e| format!("Zip error: {}", e))?;
                zip.write_all(sub.content.as_bytes())
                    .map_err(|e| format!("Zip write error: {}", e))?;
            }
            zip.finish().map_err(|e| format!("Zip finish error: {}", e))?;
        }
        Ok(buf.into_inner())
    })
    .await
    .map_err(|e| format!("Zip task failed: {}", e))??;

    std::fs::write(&zip_path, &zip_bytes)
        .map_err(|e| format!("Failed to save zip: {}", e))?;

    let result_path = zip_path.to_string_lossy().to_string();
    log::info!("[Download] Saved zip to {}", result_path);
    Ok(result_path)
}

/// Export results as CSV string. Frontend can save it via dialog.
#[tauri::command]
pub async fn export_results_csv_cmd(
    session_state: State<'_, SessionState>,
    session_id: String,
) -> Result<String, String> {
    let db = session_state.db.clone();

    let submissions = db
        .get_final_submissions(&session_id)
        .map_err(|e| e.to_string())?;

    let mut csv = String::from("student_id,filename,lang,judge_result,exec_time_ms,submitted_at\n");
    for sub in &submissions {
        csv.push_str(&format!(
            "{},{},{},{},{},{}\n",
            escape_csv(&sub.student_id),
            escape_csv(&sub.filename),
            escape_csv(sub.lang.as_deref().unwrap_or("")),
            escape_csv(judge_result_enum_to_str(&sub.judge_result)),
            sub.exec_time_ms
                .map(|ms| ms.to_string())
                .unwrap_or_default(),
            escape_csv(&sub.submitted_at.to_rfc3339()),
        ));
    }

    Ok(csv)
}

/// Returns the user's Downloads directory path.
#[tauri::command]
pub fn get_downloads_dir_cmd() -> Result<String, String> {
    dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine downloads directory".into())
}
