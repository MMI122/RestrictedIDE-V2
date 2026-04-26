# Mehedi-86 Work Division

This section describes what Mehedi-86 implemented by reading the actual source files and session modules.

## Individual Ownership (Mehedi-86)
### 1) Session UX Foundation (Frontend)
- Built the first complete exam-session flow in frontend modules, including:
- screen/state navigation for landing, create, join, dashboard, and completion flow
- session creation form with dynamic question cards and testcase modeling
- join flow with code normalization, validation, heartbeat, waiting-for-start loop, reconnect handling, and disconnect grace logic
- student question panel with markdown rendering, multi-question navigation, and allowed-URL viewer integration
- timer + auto-submit integration
- submit flow that collects allowed source files recursively and submits through LAN HTTP or local IPC
- Main implementation areas:
- `src/session.js`
- `src/session-create.js`
- `src/session-join.js`
- `src/session-dashboard.js`
- `src/session-question.js`
- `src/session-timer.js`
- `src/session-submit.js`

### 2) UI Structure and Styling System for Session Features
- Implemented the base UI structure and styling for session lifecycle screens and controls.
- Built core visual behavior for create/join forms, question cards, admin dashboard blocks, student session bar, timer states, and completion screen presentation.
- Main implementation areas:
- `src/index.html`
- `src/styles.css`

### 3) Frontend-Backend Session Integration Alignment
- Wired session modules into application startup and screen initialization flow.
- Fixed IPC payload naming and state-shape mismatches so frontend calls align with Rust command expectations.
- Fixed dashboard participant/state edge cases tied to serialized naming and null display values.
- Main implementation areas:
- `src/app.js`
- `src/index.html`
- `src/session-create.js`
- `src/session-join.js`
- `src/session-dashboard.js`
- `src/session-submit.js`

### 4) Session Backend Module Foundations: Shared Work by MMI22 and Mehedi-86.
- Co-built the initial backend session stack that powers LAN exam operations.
- The code areas include:
- command surface for create/join/start/end/submit/heartbeat/participants/submissions/violations/broadcasts
- LAN HTTP server routes and API wrappers
- SQLite schema and persistence for sessions, participants, submissions, violations, and broadcast receipts
- core session models and typed request/response contracts
- Main implementation areas:
- `src-tauri/src/commands/session_commands.rs`
- `src-tauri/src/session/db.rs`
- `src-tauri/src/session/lan_server.rs`
- `src-tauri/src/session/models.rs`
- `src-tauri/src/session/transport.rs`
- `src-tauri/src/session/mod.rs`

## Shared Work (Mehedi-86 + MMI122)
These areas were implemented jointly and should be credited to both contributors:
- Session core + LAN networking backend foundation.
- Live dashboard controls and session receipt/broadcast behavior.
- Judge/update/export pipeline integration.
- In-app allowlist documentation viewer integration (frontend + backend fetch/validation path).

## Practical Attribution Summary
Mehedi-86 primarily owned the session-facing product workflow and UI architecture, and co-built the foundational session backend pieces that made LAN-based session lifecycle features operational.
