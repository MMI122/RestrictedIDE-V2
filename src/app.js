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

const TeacherMaterials = (() => {
  let items = [];
  let activeId = null;

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

  function humanSize(bytes) {
    const n = Number(bytes || 0);
    if (!Number.isFinite(n) || n <= 0) return '0 B';
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  }

  function render() {
    const listEl = $('#teacher-file-list');
    const titleEl = $('#teacher-file-title');
    const contentEl = $('#teacher-file-content');
    const downloadBtn = $('#btn-download-teacher-file');
    if (!listEl || !titleEl || !contentEl || !downloadBtn) return;

    if (items.length === 0) {
      listEl.innerHTML = '<div class="teacher-file-empty">No teacher-shared files yet.</div>';
      titleEl.textContent = 'No file selected';
      contentEl.value = '';
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
        render();
      });
    });

    titleEl.textContent = `${active.file_name} (${humanSize(active.size)})`;
    if (active.encoding === 'text') {
      contentEl.value = active.content;
    } else {
      contentEl.value = 'Binary file received. Use Download to open it in an external app.';
    }

    downloadBtn.disabled = false;
    downloadBtn.onclick = () => {
      const blob = toBlob(active);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = active.file_name;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 3000);
    };
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
    render();
    return true;
  }

  function reset() {
    items = [];
    activeId = null;
    render();
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

/* ── Initialisation ───────────────────────────────────────────────────── */

document.addEventListener('DOMContentLoaded', async () => {
  try {
    IDE.sandboxPath = await invoke('get_sandbox_path');
    IDE.currentPath = IDE.sandboxPath;

    // Load file tree
    await FileTree.load(IDE.currentPath);

    // Set up activity bar
    setupActivityBar();
    TeacherMaterials.render();

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
          const server = Session.sessionData.server || '';
          const host = server.split(':')[0]?.toLowerCase();
          const isRemote = host && host !== 'localhost' && host !== '127.0.0.1';

          if (isRemote) {
            fetch(`http://${server}/api/session/${Session.sessionData.id}/violations`, {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({
                student_id: Session.sessionData.studentId,
                event_type: 'focus_loss',
                severity: consecutive_losses >= 3 ? 'critical' : 'warning',
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
              severity: consecutive_losses >= 3 ? 'critical' : 'warning',
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
