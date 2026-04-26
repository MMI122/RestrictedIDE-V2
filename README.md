# Restricted IDE - Project Documentation

**Project Name:** Restricted IDE (Tauri Edition)  
**Version:** 0.1.0  
**Date:** April 2026  
**Technology Stack:** Rust + Tauri v2 + Vanilla HTML/CSS/JavaScript  

---

## Table of Contents

1. [Project Overview](#project-overview)
2. [Architecture](#architecture)
3. [Technology Choices & Rationale](#technology-choices--rationale)
4. [Project Structure](#project-structure)
5. [Core Features](#core-features)
6. [Security Implementation](#security-implementation)
7. [Policy Engine](#policy-engine)
8. [Code Execution System](#code-execution-system)
9. [Build & Deployment](#build--deployment)
10. [Performance Characteristics](#performance-characteristics)
11. [Future Enhancements](#future-enhancements)

---

## Project Overview

### Purpose
A secure, restricted integrated development environment (IDE) designed for educational institutions, training centers, and controlled computing environments. The IDE provides a sandboxed coding environment with policy-driven restrictions, OS-level security enforcement, and optional kiosk mode for exam scenarios.

### Key Objectives
- **Security First**: Prevent unauthorized file access, process execution, and system manipulation
- **Minimal Footprint**: Small installer size (~3-8 MB), low RAM usage (~30-60 MB)
- **Offline-Capable**: No internet dependency for core functionality
- **Policy-Driven**: Configurable restrictions via JSON policies
- **Educational Focus**: Support for Python, JavaScript, C, C++, and Java

### Migration Context
This project is a complete rewrite from an Electron-based implementation to Tauri v2, resulting in:
- **95% smaller installer** (from ~150 MB → ~5 MB)
- **80% less RAM usage** (from ~200 MB → ~40 MB)
- **Enhanced security** (Rust memory safety + capability-based permissions)
- **Better performance** (compiled Rust vs interpreted Node.js)

---

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend (WebView2)                  │
│  ┌──────────────────────────────────────────────────┐  │
│  │  UI Layer (Vanilla HTML/CSS/JS)                  │  │
│  │  - Editor with syntax highlighting               │  │
│  │  - File tree explorer                            │  │
│  │  - Output console with stdin support             │  │
│  │  - Admin authentication dialog                   │  │
│  └──────────────────────────────────────────────────┘  │
│                         ↕ Tauri IPC                     │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Backend (Rust)                                  │  │
│  │  ┌────────────────────────────────────────────┐  │  │
│  │  │  Policy Engine                             │  │  │
│  │  │  - File access validation                  │  │  │
│  │  │  - URL/keyboard/process rules              │  │  │
│  │  │  - Time-based restrictions                 │  │  │
│  │  └────────────────────────────────────────────┘  │  │
│  │  ┌────────────────────────────────────────────┐  │  │
│  │  │  Code Execution Engine                     │  │  │
│  │  │  - Python/JS/C/C++/Java support            │  │  │
│  │  │  - Sandboxed subprocess spawning           │  │  │
│  │  │  - stdin/stdout/stderr streaming           │  │  │
│  │  └────────────────────────────────────────────┘  │  │
│  │  ┌────────────────────────────────────────────┐  │  │
│  │  │  Security Layer (Windows-specific)         │  │  │
│  │  │  - Keyboard hooks (block combos)           │  │  │
│  │  │  - Process monitor (kill blacklisted)      │  │  │
│  │  │  - Clipboard guard (auto-clear)            │  │  │
│  │  └────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Component Breakdown

#### Frontend (src/)
- **app.js**: Global state, Tauri bridge, activity bar, utilities
- **editor.js**: Tab management, code editor, syntax highlighting, line numbers
- **file-tree.js**: Directory listing, file operations (create/delete/rename)
- **code-runner.js**: Run/stop buttons, stdin input handling, output event listeners
- **search.js**: Full-text search across sandbox files
- **admin.js**: Admin authentication, session management, exit control
- **styles.css**: Dark theme, VS Code-inspired layout, responsive flex design
- **index.html**: Single-page app structure

#### Backend (src-tauri/src/)
- **lib.rs**: Entry point, app state, command registration, kiosk setup
- **config.rs**: Environment-based configuration loading
- **logger.rs**: Structured logging with debug/info/error levels
- **policy/**: Policy engine with 5 rule types (URL, keyboard, file, process, time)
- **commands/**: Tauri command handlers for FS, admin, execution, system, policy
- **security/**: Windows API hooks for keyboard, process monitoring, clipboard
- **runtime/**: Session manager with bcrypt authentication

---

## Technology Choices & Rationale

### Why Tauri v2?
- **Size**: Uses system WebView2 instead of bundling Chromium (saves ~140 MB)
- **Performance**: Rust backend compiled to native machine code
- **Security**: Memory-safe Rust + capability-based permission model
- **Modern**: Latest Tauri v2 with improved IPC and event system

### Why Rust?
- **Memory Safety**: No buffer overflows, use-after-free, or data races
- **Zero-Cost Abstractions**: High-level code, low-level performance
- **Native OS Integration**: Direct Win32 API calls via `windows` crate
- **Ecosystem**: Strong support for system programming (sysinfo, regex, etc.)

### Why Vanilla JS (No Framework)?
- **Minimal Bundle**: No React/Vue/Angular overhead (~1-5 MB saved)
- **Direct Control**: No build complexity, no transpilation needed
- **Fast Load**: Single HTML file, instant startup
- **Educational**: Easier to understand for students/auditors

### Dependency Choices

| Crate | Purpose | Justification |
|---|---|---|
| `tauri = "2"` | App framework | Latest stable v2 with event system |
| `serde + serde_json` | Serialization | De facto standard for Rust JSON |
| `bcrypt = "0.16"` | Password hashing | Industry-standard, resistant to timing attacks |
| `chrono = "0.4"` | Date/time handling | Rich datetime manipulation |
| `sysinfo = "0.33"` | System info | Cross-platform CPU/memory/process access |
| `windows = "0.58"` | Win32 API | Official Microsoft Rust bindings |
| `walkdir = "2"` | Directory traversal | Safe recursive file walking |

---

## Project Structure

```
restricted-ide-tauri/
├── src/                          # Frontend (HTML/CSS/JS)
│   ├── index.html                # Single-page app structure
│   ├── styles.css                # Dark theme, ~493 lines
│   ├── app.js                    # Global state, utilities
│   ├── editor.js                 # Code editor, tabs
│   ├── file-tree.js              # File explorer
│   ├── code-runner.js            # Code execution UI
│   ├── search.js                 # File search
│   └── admin.js                  # Admin controls
│
├── src-tauri/                    # Backend (Rust)
│   ├── Cargo.toml                # Dependencies, release optimization
│   ├── tauri.conf.json           # App metadata, window config
│   ├── build.rs                  # Tauri build script
│   │
│   ├── capabilities/             # Tauri v2 permissions
│   │   └── default.json          # Event system permissions
│   │
│   ├── icons/                    # App icons (generated)
│   │   ├── 32x32.png
│   │   ├── 128x128.png
│   │   ├── 128x128@2x.png
│   │   └── icon.ico
│   │
│   └── src/
│       ├── main.rs               # Entry point
│       ├── lib.rs                # App setup, state management
│       ├── config.rs             # Configuration loader
│       ├── logger.rs             # Logging setup
│       │
│       ├── policy/               # Policy engine (~350 lines)
│       │   ├── mod.rs
│       │   ├── engine.rs         # Main validation logic
│       │   └── rules/
│       │       ├── url_rule.rs
│       │       ├── keyboard_rule.rs
│       │       ├── file_access_rule.rs
│       │       ├── process_rule.rs
│       │       └── time_rule.rs
│       │
│       ├── commands/             # Tauri command handlers (~800 lines)
│       │   ├── mod.rs
│       │   ├── fs_commands.rs        # File system ops
│       │   ├── admin_commands.rs     # Auth & session
│       │   ├── code_execution.rs     # Run/stop code (~333 lines)
│       │   ├── system_commands.rs    # System info
│       │   └── policy_commands.rs    # Policy queries
│       │
│       ├── security/             # OS-level security (~250 lines)
│       │   ├── mod.rs
│       │   ├── keyboard_hook.rs      # WH_KEYBOARD_LL hook
│       │   ├── process_monitor.rs    # Blacklist enforcement
│       │   └── clipboard_guard.rs    # Auto-clear clipboard
│       │
│       └── runtime/              # Session management
│           ├── mod.rs
│           └── session.rs            # bcrypt auth, timeouts
│
├── dev-data/                     # Development sandbox
│   └── sandbox/                  # Default file storage
│
├── .gitignore
└── README.md
```

**Total Project Size:**
- **Source Code**: ~3,500 lines Rust + ~1,200 lines JS/CSS/HTML
- **Compiled Binary**: ~2-3 MB (release build, stripped)
- **Installer**: ~3-8 MB (includes icons, manifest)

---

## Core Features

### 1. Code Editor
- **Multi-tab interface**: Open multiple files simultaneously
- **Syntax highlighting**: Basic highlighting for JS, Python, C/C++, HTML, CSS, JSON
- **Line numbers**: Vertical gutter with line-by-line numbering
- **Auto-save support**: Ctrl+S or manual save button
- **Modified indicator**: Visual cue for unsaved changes

### 2. File Explorer
- **Tree view**: Hierarchical display of sandbox directory
- **File operations**: Create, delete, rename files and folders
- **File icons**: Visual indicators for file types (.py, .cpp, .js, etc.)
- **Sandbox-only access**: All operations restricted to configured sandbox path

### 3. Code Execution
- **Supported languages**: Python, JavaScript (Node.js), C, C++, Java
- **Compilation**: Automatic g++/gcc/javac compilation for compiled languages
- **Interactive input**: stdin bar appears for programs requiring input
- **Real-time output**: Streaming stdout/stderr via Tauri events
- **Process control**: Run, stop, send input
- **Timeout**: 120-second automatic termination
- **Error handling**: Compilation errors, runtime errors, compiler not found

### 4. Output Console
- **Resizable panel**: Drag handle to adjust height (80px - 60% viewport)
- **Color-coded output**: stdout (white), stderr (red), info (blue/italic)
- **Auto-scroll**: Always shows latest output
- **stdin input bar**: Text field + send button for interactive programs
- **Clear button**: Wipe console output

### 5. Search Functionality
- **Full-text search**: Recursively search all files in sandbox
- **Regex support**: Optional regex patterns
- **Max results limit**: Configurable result count
- **File path filtering**: Include/exclude patterns
- **Click-to-open**: Jump to matching files

### 6. Admin Authentication
- **Triple-click + Ctrl+Shift+Alt+A**: Hidden admin dialog trigger
- **bcrypt password verification**: Industry-standard password hashing
- **Session timeout**: 5-minute automatic logout
- **Lockout mechanism**: 3 failed attempts = 5-minute lockout
- **Exit control**: Only admins can close the application in kiosk mode

---

## Security Implementation

### Policy-Based Restrictions

All security rules are enforced through the **PolicyEngine**, which validates actions before execution.

#### File Access Control
```rust
// Enforced on: read_file, write_file, delete_file, run_code
- Sandbox mode: Restrict all operations to sandbox directory
- Extension filtering: Block/allow by file extension (.exe, .bat, etc.)
- Path traversal protection: Detect and block ../ and absolute paths
```

#### Keyboard Restrictions
```rust
// Enforced via: Windows WH_KEYBOARD_LL low-level keyboard hook
- Blocked combinations: Ctrl+Alt+Del, Alt+Tab, Alt+F4, Win+L, etc.
- Configurable blocklist: Environment-based key combo definitions
- Background monitoring: Hook runs on dedicated thread
```

#### Process Control
```rust
// Enforced via: Background process monitor (sysinfo polling)
- Blacklist enforcement: Auto-kill blacklisted processes every N ms
- Whitelist support: Allow only specific executables
- System process exceptions: Don't kill Windows system processes
```

#### Clipboard Protection
```rust
// Enforced via: Windows clipboard API polling
- Auto-clear: Empty clipboard every 3 seconds in kiosk mode
- No copy-paste: Prevents data exfiltration via clipboard
```

#### Time-Based Restrictions
```rust
// Enforced on: All policy validations
- Day/time schedules: Allow operations only during configured hours
- Timezone-aware: Uses system local time
```

### Kiosk Mode

When `KIOSK_ENABLED=1`:
1. **Window locked**: Fullscreen, always-on-top, no close button
2. **Keyboard blocked**: All dangerous key combos intercepted
3. **Process monitor active**: Blacklisted processes killed instantly
4. **Clipboard cleared**: Every 3 seconds
5. **Exit requires admin**: Password-protected application exit

### Environment-Based Configuration

```bash
# Core settings
KIOSK_ENABLED=1                    # Enable kiosk mode
ADMIN_PASSWORD_HASH=<bcrypt>       # Admin password

# Sandbox
SANDBOX_PATH=/path/to/sandbox      # File operations root

# Input control
BLOCKED_KEY_COMBINATIONS=ctrl+alt+del,alt+f4,win+l

# Process control
PROCESS_BLACKLIST=cmd.exe,powershell.exe,taskmgr.exe
PROCESS_WHITELIST=python.exe,node.exe,g++.exe
PROCESS_MONITOR_INTERVAL_MS=500

# File access
FILE_ALLOWED_EXTENSIONS=.py,.js,.c,.cpp,.h,.txt
FILE_DENIED_EXTENSIONS=.exe,.bat,.cmd,.ps1
FILE_SANDBOX_MODE=1

# Session
SESSION_TIMEOUT_MS=300000          # 5 minutes
SESSION_MAX_ATTEMPTS=3
SESSION_LOCKOUT_DURATION_MS=300000
```

---

## Policy Engine

### Architecture

```rust
pub struct PolicyEngine {
    pub url_rule: UrlRule,
    pub keyboard_rule: KeyboardRule,
    pub file_access_rule: FileAccessRule,
    pub process_rule: ProcessRule,
    pub time_rule: TimeRule,
}

pub struct ValidationResult {
    pub allowed: bool,
    pub reason: Option<String>,
}
```

### Rule Types

#### 1. URL Rule
- **Purpose**: Filter allowed/blocked URLs (future web browsing feature)
- **Implementation**: Glob patterns converted to regex
- **Default**: Block all URLs

#### 2. Keyboard Rule
- **Purpose**: Block dangerous keyboard combinations
- **Implementation**: Normalized combo strings (e.g., "ctrl+shift+esc")
- **Default**: Block system shortcuts (task manager, lock screen, etc.)

#### 3. File Access Rule
- **Purpose**: Sandbox file operations
- **Implementation**: Path validation, extension filtering, traversal detection
- **Default**: Sandbox-only, allow source file extensions

#### 4. Process Rule
- **Purpose**: Control executable processes
- **Implementation**: Whitelist (allow-only) or blacklist (block) mode
- **Default**: Blacklist cmd.exe, powershell.exe, regedit.exe, etc.

#### 5. Time Rule
- **Purpose**: Time-based access control
- **Implementation**: Day-of-week + hour range (e.g., Mon-Fri 09:00-17:00)
- **Default**: Allow 24/7

### Policy Validation Flow

```
User Action (e.g., write_file)
    ↓
Command Handler
    ↓
PolicyEngine.validate_file_access(path, operation)
    ↓
┌─────────────────────────────┐
│ 1. Check time restrictions  │
│ 2. Check sandbox path       │
│ 3. Check file extension     │
│ 4. Check path traversal     │
└─────────────────────────────┘
    ↓
ValidationResult { allowed, reason }
    ↓
If allowed: Execute operation
If blocked: Return error to frontend
```

---

## Code Execution System

### Supported Languages & Workflow

| Language | Compiler/Interpreter | Workflow |
|---|---|---|
| **Python** | `python` | Direct execution |
| **JavaScript** | `node` | Direct execution |
| **C** | `gcc` | Compile → Execute |
| **C++** | `g++` | Compile → Execute |
| **Java** | `javac` + `java` | Compile → Execute with classpath |

### Execution Pipeline

```rust
run_code(file_path) {
    1. Kill any existing running process
    2. Validate file access via PolicyEngine
    3. Determine language from file extension
    4. For compiled languages:
        a. Run compiler (gcc/g++/javac)
        b. Capture compilation stdout/stderr
        c. If compilation fails, emit error and exit
    5. Spawn child process with:
        - stdin: piped (for interactive input)
        - stdout: piped (for output streaming)
        - stderr: piped (for error streaming)
        - env: minimal (PATH, TEMP, HOME only)
        - cwd: file's parent directory
    6. Store process handle in AppState
    7. Spawn 3 background threads:
        a. stdout reader → emit code-output events
        b. stderr reader → emit code-output events
        c. watchdog → check process status, enforce 120s timeout
    8. Return success
}

send_code_input(text) {
    - Write text + '\n' to process stdin
    - Flush immediately
}

stop_code() {
    - Kill process via taskkill /F /T (Windows)
    - Remove from AppState
}
```

### Event-Driven Output Streaming

```javascript
// Frontend listens to Tauri events
await listen('code-output', (event) => {
    const { type, text } = event.payload;
    appendOutput(type, text);  // 'stdout' | 'stderr' | 'info'
});

await listen('code-exit', (event) => {
    const { code, signal, error } = event.payload;
    // Display exit code, cleanup UI
});
```

### Interactive Input

1. Program blocks on `cin >>` or `input()`
2. stdin bar appears in output console
3. User types input, presses Enter
4. Frontend calls `send_code_input(text)`
5. Backend writes to process stdin pipe
6. Program continues execution

### Security Constraints

- **Process timeout**: 120 seconds max (configurable `MAX_EXEC_SECS`)
- **Environment isolation**: Only essential env vars (no full PATH)
- **Working directory**: File's parent folder (can't escape sandbox)
- **No network access**: Processes don't have network permissions (OS-enforced)

---

## Build & Deployment

### Development Build

```powershell
cd restricted-ide-tauri
npx tauri dev
```

**Build time**: ~45-60 seconds (first build ~5-7 minutes)  
**Output**: Launch app in dev mode with hot reload (frontend only)

### Release Build

```powershell
npx tauri build
```

**Build time**: ~10-15 minutes (full optimization)  
**Output**:
- `src-tauri/target/release/restricted-ide.exe` — Portable binary (~2-3 MB)
- `src-tauri/target/release/bundle/msi/restricted-ide_0.1.0_x64.msi` — Windows installer (~3-8 MB)
- `src-tauri/target/release/bundle/nsis/restricted-ide_0.1.0_x64-setup.exe` — NSIS installer

### Optimization Settings

```toml
[profile.release]
strip = true           # Remove debug symbols
lto = true             # Link-time optimization
codegen-units = 1      # Single compilation unit (slower build, faster runtime)
opt-level = "s"        # Optimize for size
```

### Installer Features
- **MSI**: Standard Windows installer, supports admin deployment
- **NSIS**: Custom installer with branding, shortcuts, uninstaller
- **Portable**: Single .exe, no installation required

### System Requirements

**Runtime Dependencies:**
- Windows 10/11 (version 1809+)
- WebView2 Runtime (automatically installed on Windows 11, optional download for Win10)

**For Code Execution:**
- Python 3.x (for .py files)
- Node.js (for .js files)
- MinGW GCC/G++ (for .c/.cpp files)
- Java JDK (for .java files)

**Disk Space:**
- Application: ~10 MB
- User data: Varies (sandbox files)

---

