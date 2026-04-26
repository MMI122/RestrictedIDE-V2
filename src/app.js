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

let closeAttemptHandling = false;
let emergencyUnlockInFlight = false;
let lockdownFocusEnforceTimer = null;
const LOCKDOWN_FOCUS_ENFORCE_MS = 1500;
let lockdownBlockedComboAt = 0;
let lockdownBlockedComboName = '';
const LOCKDOWN_BLOCKED_COMBO_SUPPRESS_MS = 2200;

function getSessionSecurityMode() {
  return String(Session?.sessionData?.security?.security_mode || 'monitor').toLowerCase();
}

function isLockdownSession() {
  return getSessionSecurityMode() === 'lockdown';
}

function clearLockdownFocusEnforceTimer() {
  if (lockdownFocusEnforceTimer) {
    clearTimeout(lockdownFocusEnforceTimer);
    lockdownFocusEnforceTimer = null;
  }
}

function markLockdownBlockedCombo(name) {
  lockdownBlockedComboAt = Date.now();
  lockdownBlockedComboName = name;
}

function consumeRecentLockdownBlockedCombo() {
  const now = Date.now();
  if (now - lockdownBlockedComboAt <= LOCKDOWN_BLOCKED_COMBO_SUPPRESS_MS) {
    const name = lockdownBlockedComboName;
    lockdownBlockedComboAt = 0;
    lockdownBlockedComboName = '';
    return name || 'blocked_combo';
  }
  return null;
}

function interceptLockdownBlockedCombo(e) {
  if (!isLockdownSession() || Session?.role !== 'student') return false;

  const key = String(e.key || '').toLowerCase();
  const alt = !!e.altKey;
  const ctrl = !!e.ctrlKey;
  const shift = !!e.shiftKey;
  const win = !!e.metaKey || key === 'meta';

  let combo = null;
  if (alt && key === 'tab') combo = 'alt+tab';
  else if (alt && key === 'f4') combo = 'alt+f4';
  else if (alt && key === 'escape') combo = 'alt+escape';
  else if (ctrl && !alt && !shift && key === 'escape') combo = 'ctrl+escape';
  else if (ctrl && shift && !alt && key === 'escape') combo = 'ctrl+shift+escape';
  else if (win && !alt && !ctrl && !shift && key === 'meta') combo = 'win';
  else if (win && key === 'd') combo = 'win+d';
  else if (win && key === 'e') combo = 'win+e';
  else if (win && key === 'r') combo = 'win+r';
  else if (win && key === 'l') combo = 'win+l';
  else if (!alt && !ctrl && !shift && key === 'f11') combo = 'f11';
  else if (!alt && !ctrl && !shift && key === 'f12') combo = 'f12';
  else if (ctrl && shift && !alt && key === 'i') combo = 'ctrl+shift+i';

  if (!combo) return false;

  markLockdownBlockedCombo(combo);
  e.preventDefault();
  e.stopPropagation();
  appendOutput('info', `🛡️ [Lockdown] Blocked shortcut: ${combo}`);
  return true;
}

function isRemoteSessionServer(server) {
  if (!server) return false;
  const host = String(server).split(':')[0].toLowerCase();
  return host !== 'localhost' && host !== '127.0.0.1';
}

async function handleCloseAttemptEvent(payload) {
  if (closeAttemptHandling) return;

  if (Session?.role !== 'student' || !Session?.sessionData?.id || !Session?.sessionData?.studentId) {
    return;
  }

  closeAttemptHandling = true;
  const ts = payload?.timestamp || new Date().toISOString();
  setStatus('⛔ Close attempt blocked. Auto-submitting exam...');
  appendOutput('error', '⛔ [Security] Close attempt blocked (Alt+F4/window close). Auto-submitting.');

  const server = Session.sessionData.server || '';
  if (isRemoteSessionServer(server)) {
    try {
      await fetch(`http://${server}/api/session/${Session.sessionData.id}/violations`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          student_id: Session.sessionData.studentId,
          event_type: 'close_attempt',
          severity: 'critical',
          details: `Close attempt blocked at ${ts}`,
        }),
      });
    } catch (e) {
      console.warn('Failed to report close-attempt violation over LAN:', e);
    }
  }

  try {
    await SubmitFlow.autoSubmit();
  } catch (err) {
    console.error('Auto-submit on close-attempt failed:', err);
    appendOutput('error', '❌ Auto-submit failed after close attempt. Submit manually now.');
  } finally {
    closeAttemptHandling = false;
  }
}

async function requestEmergencyUnlock() {
  if (emergencyUnlockInFlight) return;

  if (Session?.role !== 'student' || !Session?.sessionData?.id || !Session?.sessionData?.studentId) {
    return;
  }

  if (!isLockdownSession()) {
    appendOutput('error', 'Emergency unlock is only available in lockdown mode.');
    return;
  }

  if (!Session.sessionData?.security?.lockdown_emergency_unlock) {
    appendOutput('error', 'Emergency unlock is disabled for this session.');
    return;
  }

  const password = window.prompt('Enter invigilator emergency unlock password:');
  if (password == null) return;

  const trimmed = String(password).trim();
  if (!trimmed) {
    appendOutput('error', 'Emergency unlock canceled: empty password.');
    return;
  }

  emergencyUnlockInFlight = true;
  setStatus('Validating emergency unlock...');

  try {
    let unlocked = false;
    const server = Session.sessionData.server || '';
    if (isRemoteSessionServer(server)) {
      const res = await fetch(`http://${server}/api/session/${Session.sessionData.id}/unlock`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          student_id: Session.sessionData.studentId,
          password: trimmed,
        }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok || payload.ok === false) {
        throw new Error(payload.error || `HTTP ${res.status}`);
      }
      unlocked = !!(payload.data && payload.data.unlocked);
    } else {
      unlocked = await invoke('unlock_lockdown_cmd', {
        sessionId: Session.sessionData.id,
        password: trimmed,
      });
    }

    if (!unlocked) {
      appendOutput('error', '⛔ Invalid emergency unlock password.');
      setStatus('Emergency unlock failed');
      return;
    }

    appendOutput('error', '🚨 Emergency unlock approved. Auto-submitting now.');
    setStatus('Emergency unlock approved. Submitting...');

    try {
      await invoke('set_kiosk_mode', { enabled: false });
    } catch (e) {
      console.warn('Kiosk disable warning before emergency unlock submit:', e);
    }

    await SubmitFlow.autoSubmit();
  } catch (err) {
    console.error('Emergency unlock failed:', err);
    appendOutput('error', '❌ Emergency unlock failed: ' + (err.message || err));
    setStatus('Emergency unlock error');
  } finally {
    emergencyUnlockInFlight = false;
  }
}

const TeacherMaterials = (() => {
  let items = [];
  let activeId = null;
  let activeObjectUrl = null;

  function escapeForHtml(str) {
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function isTextLikeFile(file) {
    const name = (file?.name || '').toLowerCase();
    const mime = (file?.type || '').toLowerCase();
    if (mime.startsWith('text/')) return true;
    const textExts = [
      '.txt', '.md', '.json', '.yaml', '.yml', '.xml', '.csv',
      '.js', '.ts', '.jsx', '.tsx', '.py', '.java', '.c', '.cpp', '.h', '.hpp',
      '.html', '.css', '.scss', '.sql', '.sh', '.ps1', '.rb', '.go', '.rs',
    ];
    return textExts.some((ext) => name.endsWith(ext));
  }

  function readFileAsText(file) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result || ''));
      reader.onerror = () => reject(reader.error || new Error('Failed to read file as text'));
      reader.readAsText(file);
    });
  }

  function readFileAsDataUrl(file) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result || ''));
      reader.onerror = () => reject(reader.error || new Error('Failed to read file as base64'));
      reader.readAsDataURL(file);
    });
  }

  function parseMaterialPayload(rawContent) {
    if (typeof rawContent !== 'string') return null;
    let obj;
    try {
      obj = JSON.parse(rawContent);
    } catch {
      return null;
    }
    if (obj?.kind !== 'teacher_file') return null;
    if (typeof obj.file_name !== 'string' || typeof obj.content !== 'string') return null;
    return {
      kind: 'teacher_file',
      file_name: obj.file_name,
      mime_type: typeof obj.mime_type === 'string' ? obj.mime_type : 'application/octet-stream',
      size: Number(obj.size || 0),
      encoding: obj.encoding === 'base64' ? 'base64' : 'text',
      content: obj.content,
    };
  }

  function b64ToUint8Array(base64) {
    const binary = atob(base64);
    const out = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      out[i] = binary.charCodeAt(i);
    }
    return out;
  }

  function toBlob(material) {
    if (material.encoding === 'text') {
      return new Blob([material.content], { type: material.mime_type || 'text/plain' });
    }
    return new Blob([b64ToUint8Array(material.content)], { type: material.mime_type || 'application/octet-stream' });
  }

  function currentSecurityMode() {
    return String(Session?.sessionData?.security?.security_mode || 'monitor').toLowerCase();
  }

  function isStrictOrUltra() {
    const mode = currentSecurityMode();
    return mode === 'strict' || mode === 'ultra' || mode === 'lockdown';
  }

  function clearObjectUrl() {
    if (activeObjectUrl) {
      URL.revokeObjectURL(activeObjectUrl);
      activeObjectUrl = null;
    }
  }

  function clearPreviewViews() {
    const textEl = $('#teacher-file-content');
    const pdfEl = $('#teacher-file-pdf');
    const docxEl = $('#teacher-file-docx');
    if (textEl) textEl.classList.add('hidden');
    if (pdfEl) {
      pdfEl.classList.add('hidden');
      pdfEl.src = 'about:blank';
    }
    if (docxEl) {
      docxEl.classList.add('hidden');
      docxEl.textContent = '';
    }
    clearObjectUrl();
  }

  function isDocxFilename(name) {
    return String(name || '').toLowerCase().endsWith('.docx');
  }

  async function renderDocxMaterial(active, docxEl) {
    if (active.encoding !== 'base64') {
      docxEl.textContent = 'Invalid DOCX payload.';
      return;
    }

    const bytes = Array.from(b64ToUint8Array(active.content));
    const text = await invoke('extract_docx_text_cmd', { docxBytes: bytes });
    docxEl.textContent = text || '[DOCX has no readable text content]';
  }

  function humanSize(bytes) {
    const n = Number(bytes || 0);
    if (!Number.isFinite(n) || n <= 0) return '0 B';
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  }

  async function render() {
    const listEl = $('#teacher-file-list');
    const titleEl = $('#teacher-file-title');
    const contentEl = $('#teacher-file-content');
    const pdfEl = $('#teacher-file-pdf');
    const docxEl = $('#teacher-file-docx');
    const downloadBtn = $('#btn-download-teacher-file');
    if (!listEl || !titleEl || !contentEl || !pdfEl || !docxEl || !downloadBtn) return;

    if (items.length === 0) {
      listEl.innerHTML = '<div class="teacher-file-empty">No teacher-shared files yet.</div>';
      titleEl.textContent = 'No file selected';
      clearPreviewViews();
      downloadBtn.disabled = true;
      return;
    }

    const active = items.find((x) => x.id === activeId) || items[0];
    activeId = active.id;

    listEl.innerHTML = items.map((it) => {
      const cls = it.id === activeId ? 'teacher-file-item active' : 'teacher-file-item';
      return `<button class="${cls}" data-id="${it.id}"><span class="name">${escapeForHtml(it.file_name)}</span><span class="meta">${escapeForHtml(humanSize(it.size))}</span></button>`;
    }).join('');

    listEl.querySelectorAll('.teacher-file-item').forEach((btn) => {
      btn.addEventListener('click', () => {
        activeId = btn.dataset.id || null;
        render().catch((e) => console.warn('Material render failed:', e));
      });
    });

    titleEl.textContent = `${active.file_name} (${humanSize(active.size)})`;
    clearPreviewViews();

    const strictExam = isStrictOrUltra();
    downloadBtn.disabled = strictExam;
    downloadBtn.title = strictExam
      ? 'Download is disabled in strict exam modes.'
      : '';
    downloadBtn.onclick = () => {
      if (strictExam) return;
      const blob = toBlob(active);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = active.file_name;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 3000);
    };

    const mime = String(active.mime_type || '').toLowerCase();
    const isPdf = mime.includes('pdf') || String(active.file_name || '').toLowerCase().endsWith('.pdf');
    const isDocx = mime.includes('officedocument.wordprocessingml.document') || isDocxFilename(active.file_name);

    if (active.encoding === 'text') {
      contentEl.classList.remove('hidden');
      contentEl.value = active.content;
      return;
    }

    if (isPdf) {
      const blob = toBlob(active);
      activeObjectUrl = URL.createObjectURL(blob);
      pdfEl.src = activeObjectUrl;
      pdfEl.classList.remove('hidden');
      return;
    }

    if (isDocx) {
      docxEl.classList.remove('hidden');
      docxEl.textContent = 'Parsing DOCX...';
      try {
        await renderDocxMaterial(active, docxEl);
      } catch (err) {
        docxEl.textContent = 'Failed to parse DOCX in-app.';
        console.warn('DOCX render failed:', err);
      }
      return;
    }

    docxEl.classList.remove('hidden');
    docxEl.textContent = strictExam
      ? 'This binary format is blocked in strict exam modes. Ask teacher to share PDF, DOCX, or text/code format.'
      : 'Unsupported inline preview for this binary format. Download is available in monitor mode only.';
  }

  async function buildPayloadFromFile(file) {
    const maxBytes = 768 * 1024;
    if (!file) {
      throw new Error('No file selected.');
    }
    if (file.size > maxBytes) {
      throw new Error('File too large. Keep teacher-shared files under 768 KB.');
    }

    const textLike = isTextLikeFile(file);
    let encoding = 'text';
    let content = '';

    if (textLike) {
      content = await readFileAsText(file);
      encoding = 'text';
    } else {
      const dataUrl = await readFileAsDataUrl(file);
      const b64 = dataUrl.split(',')[1] || '';
      content = b64;
      encoding = 'base64';
    }

    const payload = {
      kind: 'teacher_file',
      file_name: file.name,
      mime_type: file.type || 'application/octet-stream',
      size: file.size,
      encoding,
      content,
    };

    return JSON.stringify(payload);
  }

  function ingestBroadcast(broadcast) {
    const payload = parseMaterialPayload(broadcast?.content);
    if (!payload) return false;

    const existing = items.find((x) => x.id === broadcast.id);
    if (existing) return true;

    items.unshift({
      id: broadcast.id,
      file_name: payload.file_name,
      mime_type: payload.mime_type,
      size: payload.size,
      encoding: payload.encoding,
      content: payload.content,
      created_at: broadcast.created_at || new Date().toISOString(),
    });

    if (!activeId) activeId = broadcast.id;
    render().catch((e) => console.warn('Material render failed:', e));
    return true;
  }

  function reset() {
    items = [];
    activeId = null;
    clearObjectUrl();
    render().catch((e) => console.warn('Material render failed:', e));
  }

  return {
    buildPayloadFromFile,
    ingestBroadcast,
    reset,
    render,
  };
})();

/* ── DOM cache ────────────────────────────────────────────────────────── */

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

/* ──── Theme Management ────────────────────────────────────────────── */

const ThemeManager = (() => {
  const STORAGE_KEY = 'restricted-ide-theme';
  const LIGHT_THEME = 'light';
  const DARK_THEME = 'dark';

  function getSystemTheme() {
    // Check system preference
    if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
      return DARK_THEME;
    }
    return LIGHT_THEME;
  }

  function getSavedTheme() {
    return localStorage.getItem(STORAGE_KEY) || null;
  }

  function getCurrentTheme() {
    return document.documentElement.getAttribute('data-theme') || DARK_THEME;
  }

  function setTheme(theme) {
    const validTheme = (theme === LIGHT_THEME) ? LIGHT_THEME : DARK_THEME;
    document.documentElement.setAttribute('data-theme', validTheme);
    localStorage.setItem(STORAGE_KEY, validTheme);
    updateThemeButton();
  }

  function toggleTheme() {
    const current = getCurrentTheme();
    const newTheme = current === DARK_THEME ? LIGHT_THEME : DARK_THEME;
    setTheme(newTheme);
  }

  function updateThemeButton() {
    const btn = document.getElementById('btn-toggle-theme');
    const btnLanding = document.getElementById('btn-toggle-theme-landing');
    if (btn) {
      const current = getCurrentTheme();
      btn.textContent = current === LIGHT_THEME ? '☀️ Light' : '🌙 Dark';
      btn.title = current === LIGHT_THEME ? 'Switch to Dark Mode' : 'Switch to Light Mode';
    }
    if (btnLanding) {
      const current = getCurrentTheme();
      btnLanding.textContent = current === LIGHT_THEME ? '☀️ Light' : '🌙 Dark';
      btnLanding.title = current === LIGHT_THEME ? 'Switch to Dark Mode' : 'Switch to Light Mode';
    }
  }

  function init() {
    const saved = getSavedTheme();
    const theme = saved || getSystemTheme();
    setTheme(theme);
  }

  return { init, setTheme, toggleTheme, getCurrentTheme };
})();


/* ── Initialisation ───────────────────────────────────────────────────── */

document.addEventListener('DOMContentLoaded', async () => {
  try {
    IDE.sandboxPath = await invoke('get_sandbox_path');
    IDE.currentPath = IDE.sandboxPath;

    // Load file tree
    await FileTree.load(IDE.currentPath);

    // Set up activity bar
    setupActivityBar();
    TeacherMaterials.render().catch((e) => console.warn('Material render failed:', e));

    // Global keyboard shortcuts
    document.addEventListener('keydown', handleGlobalKeys);

    // Theme management
    ThemeManager.init();
    $('#btn-toggle-theme').addEventListener('click', () => ThemeManager.toggleTheme());
    
    const btnLandingTheme = document.getElementById('btn-toggle-theme-landing');
    if (btnLandingTheme) {
      btnLandingTheme.addEventListener('click', () => ThemeManager.toggleTheme());
    }

    // Output toggle
    $('#btn-toggle-output').addEventListener('click', toggleOutput);
    $('#btn-clear-output').addEventListener('click', () => {
      $('#output-console').innerHTML = '';
    });

    // Output panel resize
    setupOutputResize();
    setupSidebarResize();

    // Status
    setStatus('Ready');

    // ── Session system ──
    Session.init();

    // ── Security event listeners ──
    listen('security://focus-change', (event) => {
      const { has_focus, timestamp, consecutive_losses } = event.payload;

      const reportAndEnforce = () => {
        setStatus(`⚠️ Focus lost (#${consecutive_losses})`);
        appendOutput('error', '⚠️ [Security] Window focus lost — violation #' + consecutive_losses);

        if (Session?.role === 'student' && Session?.sessionData?.id && Session?.sessionData?.studentId) {
          const server = Session.sessionData.server || '';
          const host = server.split(':')[0]?.toLowerCase();
          const isRemote = host && host !== 'localhost' && host !== '127.0.0.1';
          const mode = getSessionSecurityMode();
          const instantCritical = mode === 'ultra' || mode === 'lockdown';

          if (isRemote) {
            fetch(`http://${server}/api/session/${Session.sessionData.id}/violations`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                student_id: Session.sessionData.studentId,
                event_type: 'focus_loss',
                severity: (instantCritical || consecutive_losses >= 3) ? 'critical' : 'warning',
                details: `Focus lost at ${timestamp}; consecutive=${consecutive_losses}`,
              }),
            }).catch((e) => {
              console.warn('Failed to report focus violation over LAN:', e);
            });
          } else {
            invoke('report_violation_cmd', {
              sessionId: Session.sessionData.id,
              studentId: Session.sessionData.studentId,
              eventType: 'focus_loss',
              severity: (instantCritical || consecutive_losses >= 3) ? 'critical' : 'warning',
              details: `Focus lost at ${timestamp}; consecutive=${consecutive_losses}`,
            }).catch((e) => {
              console.warn('Failed to report focus violation:', e);
            });
          }

          if (typeof JoinSession?.handleFocusSecurityEvent === 'function') {
            JoinSession.handleFocusSecurityEvent(consecutive_losses).catch((e) => {
              console.warn('Focus enforcement handler failed:', e);
            });
          }
        }
      };

      if (!has_focus) {
        if (isLockdownSession()) {
          const blockedCombo = consumeRecentLockdownBlockedCombo();
          if (blockedCombo) {
            setStatus(`Blocked shortcut: ${blockedCombo}`);
            return;
          }

          // In lockdown, ignore transient focus blips caused by key spam.
          clearLockdownFocusEnforceTimer();
          lockdownFocusEnforceTimer = setTimeout(() => {
            lockdownFocusEnforceTimer = null;
            if (!document.hasFocus()) {
              reportAndEnforce();
            }
          }, LOCKDOWN_FOCUS_ENFORCE_MS);
          return;
        }

        reportAndEnforce();
      } else {
        clearLockdownFocusEnforceTimer();
        setStatus('Focus regained');
      }
    });

    listen('security://close-attempted', (event) => {
      handleCloseAttemptEvent(event.payload).catch((e) => {
        console.warn('Close-attempt handler failed:', e);
      });
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
  if (interceptLockdownBlockedCombo(e)) {
    return;
  }

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

  // Ctrl+Shift+U -> emergency unlock (lockdown student sessions)
  if (e.ctrlKey && e.shiftKey && !e.altKey && e.key.toLowerCase() === 'u') {
    e.preventDefault();
    requestEmergencyUnlock().catch((err) => {
      console.warn('Emergency unlock shortcut failed:', err);
    });
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

/* ── Sidebar drag-to-resize ───────────────────────────────────────────── */

function setupSidebarResize() {
  const handle = $('#side-resize-handle');
  const sidePanel = $('#side-panel');
  const wrapper = $('#main-wrapper');
  if (!handle || !sidePanel || !wrapper) return;

  const saved = Number(localStorage.getItem('restrictedide.sidebarWidth'));
  if (Number.isFinite(saved) && saved >= 180 && saved <= 700) {
    sidePanel.style.width = `${saved}px`;
  }

  let startX = 0;
  let startW = 0;

  handle.addEventListener('mousedown', (e) => {
    e.preventDefault();
    startX = e.clientX;
    startW = sidePanel.getBoundingClientRect().width;
    handle.classList.add('active');
    document.body.style.cursor = 'ew-resize';

    const onMove = (ev) => {
      const delta = ev.clientX - startX;
      const maxAllowed = Math.min(700, wrapper.getBoundingClientRect().width * 0.6);
      const newW = Math.max(180, Math.min(maxAllowed, startW + delta));
      sidePanel.style.width = `${Math.floor(newW)}px`;
    };

    const onUp = () => {
      handle.classList.remove('active');
      document.body.style.cursor = '';
      const finalW = Math.floor(sidePanel.getBoundingClientRect().width);
      localStorage.setItem('restrictedide.sidebarWidth', String(finalW));
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
