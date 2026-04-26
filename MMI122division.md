# MMI122 Work Division

This section describes what MMI122 implemented by reading the actual code areas they built and evolved.

## Individual Ownership (MMI122)
### 1) Core Platform and App Wiring
- Set up and maintained the main Tauri runtime wiring that connects frontend commands to backend handlers.
- Implemented and extended command registration and app lifecycle behavior in the central backend entrypoint.
- Added student-session close interception logic that blocks window close during active exams and records a violation event.
- Main implementation areas:
- `src-tauri/src/lib.rs`
- `src-tauri/src/config.rs`
- `src-tauri/src/logger.rs`
- `src-tauri/src/runtime/session.rs`

### 2) Policy Engine and Command Enforcement Layer
- Built and iterated policy validation and secure content access behavior.
- Implemented allowlist URL fetch/validation flow for in-app documentation viewing, including redirect and origin checks.
- Extended searchable/sandbox policy tooling exposed to frontend.
- Main implementation areas:
- `src-tauri/src/policy/engine.rs`
- `src-tauri/src/policy/file_access_rule.rs`
- `src-tauri/src/policy/keyboard_rule.rs`
- `src-tauri/src/policy/process_rule.rs`
- `src-tauri/src/policy/time_rule.rs`
- `src-tauri/src/policy/url_rule.rs`
- `src-tauri/src/commands/policy_commands.rs`

### 3) Secure Runtime and Anti-Cheat Hardening
- Implemented practical exam lockdown behavior in backend and connected it to session role/security mode.
- Added student-only kiosk enable guard, preventing accidental lockdown on non-student roles.
- Implemented security mode-aware enforcement (`monitor`, `strict`, `ultra`) with different keyboard/window/clipboard behavior.
- Integrated VM detection, multi-monitor detection, screenshot prevention, focus watchdog, process monitoring, keyboard hook, and mouse confinement under one policy flow.
- Added controlled-paste-aware clipboard logic (no destructive always-wipe loop when controlled paste is expected).
- Main implementation areas:
- `src-tauri/src/commands/security_commands.rs`
- `src-tauri/src/security/vm_detection.rs`
- `src-tauri/src/security/monitor_detection.rs`
- `src-tauri/src/security/screenshot_guard.rs`
- `src-tauri/src/security/focus_watchdog.rs`
- `src-tauri/src/security/keyboard_hook.rs`
- `src-tauri/src/security/process_monitor.rs`
- `src-tauri/src/security/clipboard_guard.rs`
- `src-tauri/src/security/mouse_confinement.rs`
- `src/session-join.js`
- `src/session-create.js`
- `src/app.js`

### 4) Execution, Sandbox, and Post-Session Evaluation Pipeline
- Implemented compile/run command pathways and sandbox file operation support used by the IDE workflow.
- Built post-session judging/export pipeline behavior, including result evaluation logic and submission processing enhancements.
- Main implementation areas:
- `src-tauri/src/commands/code_execution.rs`
- `src-tauri/src/commands/fs_commands.rs`
- `src-tauri/src/commands/post_session_commands.rs`
- `src/session-post.js`

### 5) Session Reliability, LAN Flow Stabilization, and Production Fixes
- Drove reliability fixes for real LAN usage: start gating, reconnect handling, role synchronization, and session-state correctness.
- Implemented runtime role/session/student synchronization behavior needed for secure join and secure teardown.
- Added fixes around remote join behavior and final-submit/exit correctness.
- Main implementation areas:
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/session/lan_server.rs`
- `src-tauri/src/session/transport.rs`
- `src-tauri/src/session/db.rs`
- `src/session-join.js`
- `src/session-submit.js`

### 6) UX and Operational Hardening Additions
- Added late-stage interface/security UX improvements (question panel behavior, sidebar flexibility, and close-attempt handling integration).
- Added operational Windows exam-shell scripts/documentation for managed lockdown deployments.
- Main implementation areas:
- `src/index.html`
- `src/styles.css`
- `scripts/windows/enable-exam-shell.ps1`
- `scripts/windows/disable-exam-shell.ps1`
- `scripts/windows/EXAM_SHELL.md`

## Shared Work (MMI122 + Mehedi-86)
These areas were implemented jointly and should be credited to both contributors:
- Session core and LAN transport stack.
- Dashboard-level live operations and receipt/broadcast-oriented flows.
- Judge/update/export pipeline refinement.
- In-app allowlist documentation viewer integration (frontend + backend fetch/validation path).

## Practical Attribution Summary
MMI122 is the primary owner of backend-core and security enforcement behavior, and also handled many of the production hardening fixes that make exam restrictions enforceable under real session conditions.
