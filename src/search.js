// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – search.js  (search across sandbox files)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const Search = (() => {

  async function run() {
    const query = $('#search-input').value.trim();
    if (!query) return;

    const results = $('#search-results');
    results.innerHTML = '<div style="padding:8px;color:var(--text-secondary);">Searching…</div>';

    try {
      const matches = await invoke('search_in_files', { query });
      results.innerHTML = '';

      if (matches.length === 0) {
        results.innerHTML = '<div style="padding:8px;color:var(--text-secondary);">No results found.</div>';
        return;
      }

      matches.forEach(m => {
        const el = document.createElement('div');
        el.className = 'search-result';

        // Relative path
        const relPath = m.file.replace(IDE.sandboxPath, '').replace(/^[/\\]/, '');

        el.innerHTML = `
          <div class="file">${relPath}</div>
          <div class="line">Line ${m.line}</div>
          <div class="match-text">${escapeHtml(m.text.trim())}</div>
        `;

        el.addEventListener('click', async () => {
          try {
            const content = await invoke('read_file', { filePath: m.file });
            const name = m.file.split(/[/\\]/).pop();
            Editor.openFile(m.file, name, content);
          } catch (e) {
            setStatus('Cannot open: ' + e);
          }
        });

        results.appendChild(el);
      });

      setStatus(`Found ${matches.length} result(s)`);
    } catch (e) {
      results.innerHTML = `<div style="padding:8px;color:var(--error);">Error: ${e}</div>`;
    }
  }

  function escapeHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  /* ── Wire ───────────────────────────────────────────────────────────── */

  document.addEventListener('DOMContentLoaded', () => {
    $('#btn-search').addEventListener('click', run);
    $('#search-input').addEventListener('keydown', (e) => {
      if (e.key === 'Enter') run();
    });
  });

  return { run };
})();
