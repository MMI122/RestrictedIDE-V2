// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – editor.js  (Monaco preserve-design integration)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const Editor = (() => {
  const textarea = () => $('#code-editor'); // hidden compatibility bridge
  const container = () => $('#editor-container');
  const welcome = () => $('#welcome');
  const tabBar = () => $('#tab-bar');
  const monacoHost = () => $('#monaco-editor');

  let monacoEditor = null;
  let monacoReady = null;
  let viewToken = 0;

  function getExt(name) {
    if (!name) return '';
    const i = name.lastIndexOf('.');
    return i >= 0 ? name.slice(i).toLowerCase() : '';
  }

  function monacoLanguageForFile(name) {
    const ext = getExt(name);
    switch (ext) {
      case '.js': return 'javascript';
      case '.ts': return 'typescript';
      case '.jsx': return 'javascript';
      case '.tsx': return 'typescript';
      case '.py': return 'python';
      case '.c': return 'c';
      case '.cpp': return 'cpp';
      case '.h': return 'cpp';
      case '.hpp': return 'cpp';
      case '.java': return 'java';
      case '.json': return 'json';
      case '.html': return 'html';
      case '.css': return 'css';
      case '.scss': return 'scss';
      case '.md': return 'markdown';
      case '.xml': return 'xml';
      case '.yaml': return 'yaml';
      case '.yml': return 'yaml';
      case '.sql': return 'sql';
      case '.sh': return 'shell';
      case '.ps1': return 'powershell';
      case '.go': return 'go';
      case '.rs': return 'rust';
      case '.php': return 'php';
      case '.rb': return 'ruby';
      default: return 'plaintext';
    }
  }

  function syncHiddenTextarea() {
    const ta = textarea();
    if (!ta) return;
    ta.value = getCurrentContent();
  }

  function getCurrentContent() {
    if (monacoEditor) return monacoEditor.getValue();
    return textarea()?.value || '';
  }

  function setCurrentContent(content, name) {
    if (monacoEditor) {
      monacoEditor.setValue(content || '');
      const model = monacoEditor.getModel();
      if (model && window.monaco?.editor) {
        window.monaco.editor.setModelLanguage(model, monacoLanguageForFile(name));
      }
    } else if (textarea()) {
      textarea().value = content || '';
    }
    syncHiddenTextarea();
  }

  function updateCursorPos() {
    if (!monacoEditor) {
      const ta = textarea();
      if (!ta) return;
      const val = ta.value.substring(0, ta.selectionStart);
      const lines = val.split('\n');
      const ln = lines.length;
      const col = lines[lines.length - 1].length + 1;
      $('#status-line').textContent = `Ln ${ln}, Col ${col}`;
      return;
    }

    const pos = monacoEditor.getPosition();
    if (!pos) return;
    $('#status-line').textContent = `Ln ${pos.lineNumber}, Col ${pos.column}`;
  }

  function applyMonacoTheme() {
    const root = getComputedStyle(document.documentElement);
    const bg = (root.getPropertyValue('--bg-primary') || '#1e1e1e').trim();
    const fg = (root.getPropertyValue('--text-primary') || '#d4d4d4').trim();
    const accent = (root.getPropertyValue('--accent') || '#569cd6').trim();

    window.monaco.editor.defineTheme('ride-preserve', {
      base: 'vs-dark',
      inherit: true,
      rules: [],
      colors: {
        'editor.background': bg,
        'editor.foreground': fg,
        'editor.lineHighlightBackground': '#ffffff0d',
        'editorCursor.foreground': accent,
        'editorLineNumber.foreground': '#7a7a7a',
        'editorLineNumber.activeForeground': '#cfcfcf',
        'editor.selectionBackground': '#264f78',
        'editor.inactiveSelectionBackground': '#3a3d41',
      },
    });

    window.monaco.editor.setTheme('ride-preserve');
  }

  function initMonaco() {
    if (monacoReady) return monacoReady;

    monacoReady = new Promise((resolve) => {
      if (!window.require || !monacoHost()) {
        console.warn('[Editor] Monaco loader not available, using fallback textarea mode.');
        resolve(false);
        return;
      }

      window.require.config({ paths: { vs: 'vendor/vs' } });
      window.require(['vs/editor/editor.main'], () => {
        try {
          applyMonacoTheme();

          monacoEditor = window.monaco.editor.create(monacoHost(), {
            value: '',
            language: 'plaintext',
            automaticLayout: true,
            fontFamily: 'Consolas, "Courier New", monospace',
            fontSize: 14,
            lineHeight: 22,
            minimap: { enabled: false },
            glyphMargin: false,
            scrollBeyondLastLine: false,
            renderLineHighlight: 'line',
            wordWrap: 'off',
            tabSize: 4,
            insertSpaces: true,
            detectIndentation: false,
            roundedSelection: false,
            stickyScroll: { enabled: false },
            bracketPairColorization: { enabled: true },
            guides: {
              indentation: true,
              bracketPairs: true,
            },
            quickSuggestions: true,
            suggestOnTriggerCharacters: true,
            parameterHints: { enabled: true },
          });

          monacoEditor.onDidChangeModelContent(() => {
            if (IDE.activeTab !== null && IDE.openTabs[IDE.activeTab]) {
              const tab = IDE.openTabs[IDE.activeTab];
              tab.content = monacoEditor.getValue();
              tab.modified = tab.content !== tab.savedContent;
              renderTabs();
            }
            syncHiddenTextarea();
            updateCursorPos();
          });

          monacoEditor.onDidChangeCursorPosition(() => updateCursorPos());
          syncHiddenTextarea();
          resolve(true);
        } catch (e) {
          console.warn('[Editor] Monaco initialization failed:', e);
          resolve(false);
        }
      }, () => {
        console.warn('[Editor] Monaco module load failed, using fallback textarea mode.');
        resolve(false);
      });
    });

    return monacoReady;
  }

  function openFile(path, name, content) {
    const idx = IDE.openTabs.findIndex((t) => t.path === path);
    if (idx >= 0) {
      switchTab(idx);
      return;
    }

    IDE.openTabs.push({ path, name, content, savedContent: content, modified: false });
    IDE.activeTab = IDE.openTabs.length - 1;
    renderTabs();
    showEditor(content, name);
  }

  function switchTab(idx) {
    if (idx < 0 || idx >= IDE.openTabs.length) return;

    if (IDE.activeTab !== null && IDE.openTabs[IDE.activeTab]) {
      IDE.openTabs[IDE.activeTab].content = getCurrentContent();
    }

    IDE.activeTab = idx;
    const tab = IDE.openTabs[idx];
    renderTabs();
    showEditor(tab.content, tab.name);
  }

  function closeTab(idxOrPath) {
    let idx = idxOrPath;
    if (typeof idxOrPath === 'string') {
      idx = IDE.openTabs.findIndex((t) => t.path === idxOrPath);
    }
    if (!Number.isInteger(idx) || idx < 0 || idx >= IDE.openTabs.length) return;

    const tab = IDE.openTabs[idx];
    if (tab.modified) {
      if (confirm(`Save changes to ${tab.name}?`)) {
        switchTab(idx);
        save();
      }
    }

    IDE.openTabs.splice(idx, 1);
    if (IDE.openTabs.length === 0) {
      IDE.activeTab = null;
      hideEditor();
    } else {
      IDE.activeTab = Math.min(idx, IDE.openTabs.length - 1);
      switchTab(IDE.activeTab);
    }
    renderTabs();
  }

  function renderTabs() {
    const bar = tabBar();
    bar.innerHTML = '';
    IDE.openTabs.forEach((tab, i) => {
      const el = document.createElement('div');
      el.className = 'tab' + (i === IDE.activeTab ? ' active' : '');

      const dot = document.createElement('span');
      dot.className = 'dot' + (tab.modified ? ' modified' : '');
      el.appendChild(dot);

      const label = document.createElement('span');
      label.textContent = tab.name;
      el.appendChild(label);

      const close = document.createElement('span');
      close.className = 'close-btn';
      close.textContent = '×';
      close.addEventListener('click', (e) => { e.stopPropagation(); closeTab(i); });
      el.appendChild(close);

      el.addEventListener('click', () => switchTab(i));
      bar.appendChild(el);
    });
  }

  function showEditor(content, name) {
    const token = ++viewToken;
    container().style.display = 'flex';
    container().classList.add('monaco-mode');
    welcome().classList.add('hidden');

    if (textarea()) textarea().value = content || '';
    setLanguageStatus(langFromExt(getExt(name)));

    initMonaco().then((ok) => {
      if (token !== viewToken || !ok || !monacoEditor) return;
      setCurrentContent(content || '', name || '');
      monacoEditor.focus();
      updateCursorPos();
    });
  }

  function hideEditor() {
    container().style.display = 'none';
    welcome().classList.remove('hidden');
    setLanguageStatus('');
    $('#status-line').textContent = '';

    if (textarea()) textarea().value = '';
    if (monacoEditor) {
      monacoEditor.setValue('');
    }
  }

  function clearCurrentEditor() {
    if (monacoEditor) {
      monacoEditor.setValue('');
    }
    if (textarea()) textarea().value = '';
  }

  async function save() {
    if (IDE.activeTab === null) return;
    const tab = IDE.openTabs[IDE.activeTab];
    tab.content = getCurrentContent();

    try {
      await invoke('write_file', { filePath: tab.path, content: tab.content });
      tab.savedContent = tab.content;
      tab.modified = false;
      renderTabs();
      setStatus(`Saved: ${tab.name}`);
    } catch (e) {
      setStatus(`Save failed: ${e}`);
    }
  }

  return {
    openFile,
    switchTab,
    closeTab,
    save,
    renderTabs,
    getCurrentContent,
    clearCurrentEditor,
  };
})();

document.addEventListener('DOMContentLoaded', () => {
  $('#btn-save').addEventListener('click', () => Editor.save());
});
