// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – app.js  (global state, Tauri bridge, utilities)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

/* ── Tauri bridge ─────────────────────────────────────────────────────── */

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

/* ── Global IDE state ─────────────────────────────────────────────────── */

const IDE = {
  sandboxPath: '',
  currentPath: '',         // currently displayed directory
  openTabs: [],            // { path, name, content, modified }
  activeTab: null,         // index into openTabs
  outputVisible: true,
};

/* ── DOM cache ────────────────────────────────────────────────────────── */

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

/* ── Initialisation ───────────────────────────────────────────────────── */

document.addEventListener('DOMContentLoaded', async () => {
  try {
    IDE.sandboxPath = await invoke('get_sandbox_path');
    IDE.currentPath = IDE.sandboxPath;

    // Load file tree
    await FileTree.load(IDE.currentPath);

    // Set up activity bar
    setupActivityBar();

    // Global keyboard shortcuts
    document.addEventListener('keydown', handleGlobalKeys);

    // Output toggle
    $('#btn-toggle-output').addEventListener('click', toggleOutput);
    $('#btn-clear-output').addEventListener('click', () => {
      $('#output-console').innerHTML = '';
    });

    // Output panel resize
    setupOutputResize();

    // Status
    setStatus('Ready');

    // ── Session system ──
    Session.init();

    // ── Security event listeners ──
    listen('security://focus-change', (event) => {
      const { has_focus, timestamp, consecutive_losses } = event.payload;
      if (!has_focus) {
        setStatus(`⚠️ Focus lost (#${consecutive_losses})`);
        appendOutput('error', '⚠️ [Security] Window focus lost — violation #' + consecutive_losses);

        if (Session?.role === 'student' && Session?.sessionData?.id && Session?.sessionData?.studentId) {
          invoke('report_violation_cmd', {
            sessionId: Session.sessionData.id,
            studentId: Session.sessionData.studentId,
            eventType: 'focus_loss',
            severity: consecutive_losses >= 3 ? 'critical' : 'warning',
            details: `Focus lost at ${timestamp}; consecutive=${consecutive_losses}`,
          }).catch((e) => {
            console.warn('Failed to report focus violation:', e);
          });
        }
      } else {
        setStatus('Focus regained');
      }
    });
  } catch (e) {
    console.error('Init error:', e);
    setStatus('Init error: ' + e);
  }
});

/* ── Activity bar (panel switching) ───────────────────────────────────── */

function setupActivityBar() {
  $$('.activity-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const panel = btn.dataset.panel;
      $$('.activity-btn').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      $$('.panel-view').forEach(p => p.classList.remove('active'));
      const target = $(`#panel-${panel}`) || $(`#panel-${panel}-side`);
      if (target) target.classList.add('active');
    });
  });
}

/* ── Global keyboard shortcuts ────────────────────────────────────────── */

function handleGlobalKeys(e) {
  // Ctrl+S → save
  if (e.ctrlKey && e.key === 's') {
    e.preventDefault();
    Editor.save();
  }
  // Ctrl+Enter → run
  if (e.ctrlKey && e.key === 'Enter') {
    e.preventDefault();
    CodeRunner.run();
  }
  // Ctrl+Shift+Alt+A → admin
  if (e.ctrlKey && e.shiftKey && e.altKey && e.key.toLowerCase() === 'a') {
    e.preventDefault();
    Admin.showDialog();
  }
}

/* ── Output panel ─────────────────────────────────────────────────────── */

function toggleOutput() {
  IDE.outputVisible = !IDE.outputVisible;
  const bar = $('#output-bar');
  if (IDE.outputVisible) {
    bar.style.display = 'flex';
  } else {
    bar.style.display = 'none';
  }
}

/* ── Output panel drag-to-resize ──────────────────────────────────────── */

function setupOutputResize() {
  const handle = $('#output-resize-handle');
  const bar = $('#output-bar');
  let startY, startH;

  handle.addEventListener('mousedown', (e) => {
    e.preventDefault();
    startY = e.clientY;
    startH = bar.offsetHeight;
    handle.classList.add('active');
    document.body.style.cursor = 'ns-resize';

    const onMove = (e) => {
      const delta = startY - e.clientY;           // dragging up = bigger
      const newH = Math.max(80, Math.min(window.innerHeight * 0.6, startH + delta));
      bar.style.height = newH + 'px';
    };
    const onUp = () => {
      handle.classList.remove('active');
      document.body.style.cursor = '';
      document.removeEventListener('mousemove', onMove);
      document.removeEventListener('mouseup', onUp);
    };
    document.addEventListener('mousemove', onMove);
    document.addEventListener('mouseup', onUp);
  });
}

/* ── Utilities ────────────────────────────────────────────────────────── */

/** Append coloured text to the output console. */
function appendOutput(type, text) {
  const el = document.createElement('span');
  el.className = `out-${type}`;
  el.textContent = text;
  const console = $('#output-console');
  console.appendChild(el);
  console.scrollTop = console.scrollHeight;
}

function setStatus(text) {
  $('#status-left').textContent = text;
}

function setLanguageStatus(lang) {
  $('#status-lang').textContent = lang;
}

/** Map file extension to a language name. */
function langFromExt(ext) {
  const map = {
    '.py': 'Python', '.js': 'JavaScript', '.ts': 'TypeScript',
    '.c': 'C', '.cpp': 'C++', '.h': 'C Header', '.hpp': 'C++ Header',
    '.java': 'Java', '.html': 'HTML', '.css': 'CSS', '.scss': 'SCSS',
    '.json': 'JSON', '.md': 'Markdown', '.txt': 'Plain Text',
    '.xml': 'XML', '.yaml': 'YAML', '.yml': 'YAML',
  };
  return map[ext] || 'Plain Text';
}

function getExt(name) {
  const dot = name.lastIndexOf('.');
  return dot >= 0 ? name.substring(dot).toLowerCase() : '';
}

/** Show a custom prompt dialog. Returns the entered value or null. */
function showPrompt(title, defaultValue = '') {
  return new Promise((resolve) => {
    const overlay = $('#prompt-overlay');
    const input = $('#prompt-input');
    const ok = $('#btn-prompt-ok');
    const cancel = $('#btn-prompt-cancel');
    $('#prompt-title').textContent = title;
    input.value = defaultValue;
    overlay.classList.remove('hidden');
    input.focus();

    function cleanup(val) {
      overlay.classList.add('hidden');
      ok.removeEventListener('click', onOk);
      cancel.removeEventListener('click', onCancel);
      input.removeEventListener('keydown', onKey);
      resolve(val);
    }
    function onOk() { cleanup(input.value); }
    function onCancel() { cleanup(null); }
    function onKey(e) {
      if (e.key === 'Enter') onOk();
      if (e.key === 'Escape') onCancel();
    }
    ok.addEventListener('click', onOk);
    cancel.addEventListener('click', onCancel);
    input.addEventListener('keydown', onKey);
  });
}

/** Get the icon for a file/folder. */
function fileIcon(name, isDir) {
  if (isDir) return '📁';
  const ext = getExt(name);
  const icons = {
    '.py': '🐍', '.js': '📜', '.ts': '📘', '.java': '☕',
    '.c': '⚙️', '.cpp': '⚙️', '.h': '📎', '.hpp': '📎',
    '.html': '🌐', '.css': '🎨', '.json': '📋', '.md': '📝',
    '.txt': '📄', '.xml': '📰', '.yaml': '⚙️', '.yml': '⚙️',
  };
  return icons[ext] || '📄';
}
