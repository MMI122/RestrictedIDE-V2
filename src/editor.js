// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – editor.js  (code editor, tabs, syntax highlighting)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const Editor = (() => {

  /* ── DOM refs ───────────────────────────────────────────────────────── */

  const textarea   = () => $('#code-editor');
  const highlight  = () => $('#syntax-highlight');
  const lineNums   = () => $('#line-numbers');
  const container  = () => $('#editor-container');
  const welcome    = () => $('#welcome');
  const tabBar     = () => $('#tab-bar');

  /* ── Tab management ─────────────────────────────────────────────────── */

  function openFile(path, name, content) {
    // Already open?
    const idx = IDE.openTabs.findIndex(t => t.path === path);
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

    // Save current editor state
    if (IDE.activeTab !== null && IDE.openTabs[IDE.activeTab]) {
      IDE.openTabs[IDE.activeTab].content = textarea().value;
    }

    IDE.activeTab = idx;
    const tab = IDE.openTabs[idx];
    renderTabs();
    showEditor(tab.content, tab.name);
  }

  function closeTab(idx) {
    const tab = IDE.openTabs[idx];
    if (tab.modified) {
      if (!confirm(`Save changes to ${tab.name}?`)) {
        // discard
      } else {
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

  /* ── Editor display ─────────────────────────────────────────────────── */

  function showEditor(content, name) {
    container().style.display = 'flex';
    welcome().classList.add('hidden');

    const ta = textarea();
    ta.value = content;
    updateLineNumbers(content);
    updateHighlight(content, name);
    setLanguageStatus(langFromExt(getExt(name)));

    // Sync scroll
    ta.onscroll = syncScroll;
    ta.oninput = () => {
      const val = ta.value;
      if (IDE.activeTab !== null) {
        const tab = IDE.openTabs[IDE.activeTab];
        tab.content = val;
        tab.modified = val !== tab.savedContent;
        renderTabs();
      }
      updateLineNumbers(val);
      updateHighlight(val, IDE.openTabs[IDE.activeTab]?.name || '');
    };

    // Cursor position update
    ta.addEventListener('click', updateCursorPos);
    ta.addEventListener('keyup', updateCursorPos);

    // Tab key support
    ta.addEventListener('keydown', (e) => {
      if (e.key === 'Tab') {
        e.preventDefault();
        const start = ta.selectionStart;
        const end = ta.selectionEnd;
        ta.value = ta.value.substring(0, start) + '    ' + ta.value.substring(end);
        ta.selectionStart = ta.selectionEnd = start + 4;
        ta.dispatchEvent(new Event('input'));
      }
    });

    ta.focus();
  }

  function hideEditor() {
    container().style.display = 'none';
    welcome().classList.remove('hidden');
    setLanguageStatus('');
    $('#status-line').textContent = '';
  }

  function syncScroll() {
    const ta = textarea();
    highlight().scrollTop = ta.scrollTop;
    highlight().scrollLeft = ta.scrollLeft;
    lineNums().scrollTop = ta.scrollTop;
  }

  function updateCursorPos() {
    const ta = textarea();
    const val = ta.value.substring(0, ta.selectionStart);
    const lines = val.split('\n');
    const ln = lines.length;
    const col = lines[lines.length - 1].length + 1;
    $('#status-line').textContent = `Ln ${ln}, Col ${col}`;
  }

  /* ── Line numbers ───────────────────────────────────────────────────── */

  function updateLineNumbers(content) {
    const count = content.split('\n').length;
    const nums = lineNums();
    let html = '';
    for (let i = 1; i <= count; i++) {
      html += i + '\n';
    }
    nums.textContent = html;
  }

  /* ── Syntax highlighting (basic) ────────────────────────────────────── */

  function updateHighlight(code, name) {
    const ext = getExt(name);
    const hl = highlight();

    // Escape HTML
    let html = code
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');

    // Apply language-specific coloring
    if (['.js', '.ts', '.jsx', '.tsx'].includes(ext)) {
      html = highlightJS(html);
    } else if (ext === '.py') {
      html = highlightPy(html);
    } else if (['.c', '.cpp', '.h', '.hpp', '.java'].includes(ext)) {
      html = highlightC(html);
    } else if (ext === '.html') {
      html = highlightHTML(html);
    } else if (ext === '.css' || ext === '.scss') {
      html = highlightCSS(html);
    } else if (ext === '.json') {
      html = highlightJSON(html);
    }

    hl.innerHTML = html + '\n'; // trailing newline for height
  }

  // Minimal keyword-based highlighting

  function highlightJS(html) {
    // Comments
    html = html.replace(/(\/\/.*)/g, '<span style="color:#6a9955">$1</span>');
    html = html.replace(/(\/\*[\s\S]*?\*\/)/g, '<span style="color:#6a9955">$1</span>');
    // Strings
    html = html.replace(/(&#39;[^&#]*?&#39;|&quot;[^&]*?&quot;|`[^`]*?`)/g, '<span style="color:#ce9178">$1</span>');
    // Keywords
    const kw = /\b(const|let|var|function|return|if|else|for|while|do|switch|case|break|continue|class|extends|import|export|from|default|new|this|async|await|try|catch|throw|typeof|instanceof|in|of|true|false|null|undefined|void)\b/g;
    html = html.replace(kw, '<span style="color:#569cd6">$1</span>');
    // Numbers
    html = html.replace(/\b(\d+\.?\d*)\b/g, '<span style="color:#b5cea8">$1</span>');
    return html;
  }

  function highlightPy(html) {
    html = html.replace(/(#.*)/g, '<span style="color:#6a9955">$1</span>');
    html = html.replace(/(&#39;&#39;&#39;[\s\S]*?&#39;&#39;&#39;|&quot;&quot;&quot;[\s\S]*?&quot;&quot;&quot;)/g, '<span style="color:#6a9955">$1</span>');
    html = html.replace(/(&#39;[^&#]*?&#39;|&quot;[^&]*?&quot;)/g, '<span style="color:#ce9178">$1</span>');
    const kw = /\b(def|class|return|if|elif|else|for|while|import|from|as|try|except|finally|raise|with|yield|lambda|and|or|not|is|in|True|False|None|pass|break|continue|global|nonlocal|assert|del|print|self)\b/g;
    html = html.replace(kw, '<span style="color:#569cd6">$1</span>');
    html = html.replace(/\b(\d+\.?\d*)\b/g, '<span style="color:#b5cea8">$1</span>');
    return html;
  }

  function highlightC(html) {
    html = html.replace(/(\/\/.*)/g, '<span style="color:#6a9955">$1</span>');
    html = html.replace(/(\/\*[\s\S]*?\*\/)/g, '<span style="color:#6a9955">$1</span>');
    html = html.replace(/(&#39;[^&#]*?&#39;|&quot;[^&]*?&quot;)/g, '<span style="color:#ce9178">$1</span>');
    const kw = /\b(int|float|double|char|void|long|short|unsigned|signed|struct|enum|union|typedef|const|static|extern|return|if|else|for|while|do|switch|case|break|continue|sizeof|include|define|ifndef|endif|class|public|private|protected|virtual|override|template|namespace|using|new|delete|try|catch|throw|string|bool|true|false|null|nullptr|auto|System|out|println|main|import|package)\b/g;
    html = html.replace(kw, '<span style="color:#569cd6">$1</span>');
    html = html.replace(/(#\w+)/g, '<span style="color:#c586c0">$1</span>');
    html = html.replace(/\b(\d+\.?\d*)\b/g, '<span style="color:#b5cea8">$1</span>');
    return html;
  }

  function highlightHTML(html) {
    html = html.replace(/(&lt;!--[\s\S]*?--&gt;)/g, '<span style="color:#6a9955">$1</span>');
    html = html.replace(/(&lt;\/?)([\w-]+)/g, '<span style="color:#808080">$1</span><span style="color:#569cd6">$2</span>');
    html = html.replace(/([\w-]+)(=)/g, '<span style="color:#9cdcfe">$1</span>$2');
    html = html.replace(/(=)(&#39;[^&#]*?&#39;|&quot;[^&]*?&quot;)/g, '$1<span style="color:#ce9178">$2</span>');
    return html;
  }

  function highlightCSS(html) {
    html = html.replace(/(\/\*[\s\S]*?\*\/)/g, '<span style="color:#6a9955">$1</span>');
    html = html.replace(/([\w-]+)\s*:/g, '<span style="color:#9cdcfe">$1</span>:');
    html = html.replace(/(#[\da-fA-F]{3,8})\b/g, '<span style="color:#ce9178">$1</span>');
    html = html.replace(/\b(\d+\.?\d*(px|em|rem|%|vh|vw|s|ms)?)\b/g, '<span style="color:#b5cea8">$1</span>');
    return html;
  }

  function highlightJSON(html) {
    html = html.replace(/(&#39;[^&#]*?&#39;|&quot;[^&]*?&quot;)\s*:/g, '<span style="color:#9cdcfe">$1</span>:');
    html = html.replace(/:\s*(&#39;[^&#]*?&#39;|&quot;[^&]*?&quot;)/g, ': <span style="color:#ce9178">$1</span>');
    html = html.replace(/\b(true|false|null)\b/g, '<span style="color:#569cd6">$1</span>');
    html = html.replace(/\b(\d+\.?\d*)\b/g, '<span style="color:#b5cea8">$1</span>');
    return html;
  }

  /* ── Save ────────────────────────────────────────────────────────────── */

  async function save() {
    if (IDE.activeTab === null) return;
    const tab = IDE.openTabs[IDE.activeTab];
    tab.content = textarea().value;

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

  /* ── Public API ─────────────────────────────────────────────────────── */

  return { openFile, switchTab, closeTab, save, renderTabs };
})();

// Wire up save button
document.addEventListener('DOMContentLoaded', () => {
  $('#btn-save').addEventListener('click', () => Editor.save());
});
