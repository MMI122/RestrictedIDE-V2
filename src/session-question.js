// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-question.js  (Question Panel for Students)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const QuestionPanel = (() => {
  let questions = [];
  let allowedUrls = [];
  let currentIdx = 0;
  let panelCollapsed = false;

  function init() {
    // Toggle panel
    $('#btn-toggle-question')?.addEventListener('click', togglePanel);
    $('#btn-open-question-panel')?.addEventListener('click', expandPanel);
    $('#btn-close-doc-viewer')?.addEventListener('click', closeDocViewer);
  }

  function togglePanel() {
    if (panelCollapsed) {
      expandPanel();
    } else {
      collapsePanel();
    }
  }

  function collapsePanel() {
    const panel = $('#question-panel');
    const btn = $('#btn-toggle-question');
    const openBtn = $('#btn-open-question-panel');
    if (!panel) return;

    panelCollapsed = true;
    panel.classList.add('collapsed');
    if (btn) btn.textContent = '▶';
    if (openBtn) openBtn.classList.remove('hidden');
  }

  function expandPanel() {
    const panel = $('#question-panel');
    const btn = $('#btn-toggle-question');
    const openBtn = $('#btn-open-question-panel');
    if (!panel) return;

    panelCollapsed = false;
    panel.classList.remove('collapsed');
    if (btn) btn.textContent = '◀';
    if (openBtn) openBtn.classList.add('hidden');
  }

  function loadQuestions(questionList, allowedUrlList = []) {
    questions = questionList || [];
    allowedUrls = allowedUrlList || [];
    currentIdx = 0;
    expandPanel();

    if (questions.length === 0) {
      renderEmpty();
      return;
    }

    renderQuestion(currentIdx);
    renderNav();
  }

  function renderQuestion(idx) {
    const container = $('#question-content');
    if (!container || !questions[idx]) return;

    const q = questions[idx];

    let html = `<h3>${escapeHtml(q.title)}</h3>`;
    html += renderAllowedUrls();

    // Simple markdown-like rendering for description
    html += `<div class="question-description">${renderMarkdown(q.description)}</div>`;

    if (q.sample_input) {
      html += `
        <div class="question-section">
          <strong>Sample Input:</strong>
          <pre>${escapeHtml(q.sample_input)}</pre>
        </div>
      `;
    }

    if (q.expected_output) {
      html += `
        <div class="question-section">
          <strong>Expected Output:</strong>
          <pre>${escapeHtml(q.expected_output)}</pre>
        </div>
      `;
    }

    // Navigation between questions
    if (questions.length > 1) {
      html += `<div class="question-nav" id="question-nav"></div>`;
    }

    container.innerHTML = html;

    // Add nav buttons
    if (questions.length > 1) {
      renderNav();
    }

    wireAllowedUrlLinks();
  }

  function renderAllowedUrls() {
    if (!Array.isArray(allowedUrls) || allowedUrls.length === 0) {
      return '';
    }

    const links = allowedUrls.map((url, i) => {
      const safeUrl = escapeHtml(url);
      return `<button class="allowed-url-link" data-url="${safeUrl}" title="${safeUrl}">Doc ${i + 1}</button>`;
    }).join('');

    return `
      <div class="allowed-urls-wrap">
        <div class="allowed-urls-title">Allowed URLs</div>
        <div class="allowed-urls-list">${links}</div>
      </div>
    `;
  }

  function normalizeUrl(u) {
    return String(u || '').trim();
  }

  function isAllowedBySession(url) {
    const normalized = normalizeUrl(url);
    return allowedUrls.some(pattern => {
      const p = normalizeUrl(pattern);
      if (!p) return false;
      if (p.endsWith('*')) {
        return normalized.startsWith(p.slice(0, -1));
      }
      return normalized === p;
    });
  }

  function wireAllowedUrlLinks() {
    document.querySelectorAll('.allowed-url-link').forEach(el => {
      el.addEventListener('click', async () => {
        const url = el.dataset.url || '';
        if (!url) return;

        if (!isAllowedBySession(url)) {
          alert('This URL is not allowed for this session.');
          return;
        }

        try {
          await openDocViewer(url);
          appendOutput('info', `Opened allowed URL in viewer: ${url}`);
        } catch (err) {
          console.error('Failed to open allowed URL:', err);
          alert('Unable to open URL: ' + (err.message || err));
        }
      });
    });
  }

  async function openDocViewer(url) {
    const viewer = $('#doc-viewer');
    const titleEl = $('#doc-viewer-title');
    const urlEl = $('#doc-viewer-url');
    const loadingEl = $('#doc-viewer-loading');
    const contentEl = $('#doc-viewer-content');
    if (!viewer || !titleEl || !urlEl || !loadingEl || !contentEl) {
      throw new Error('Docs viewer UI is not available');
    }

    viewer.classList.remove('hidden');
    loadingEl.classList.remove('hidden');
    contentEl.textContent = '';
    titleEl.textContent = 'Loading...';
    urlEl.textContent = url;

    const data = await invoke('fetch_allowed_doc_cmd', { url });

    titleEl.textContent = data?.title || 'Documentation';
    urlEl.textContent = data?.url || url;
    contentEl.textContent = data?.content || 'No content available.';
    loadingEl.classList.add('hidden');
  }

  function closeDocViewer() {
    const viewer = $('#doc-viewer');
    const contentEl = $('#doc-viewer-content');
    const loadingEl = $('#doc-viewer-loading');
    if (viewer) viewer.classList.add('hidden');
    if (contentEl) contentEl.textContent = '';
    if (loadingEl) loadingEl.classList.add('hidden');
  }

  function renderNav() {
    const nav = $('#question-nav');
    if (!nav || questions.length <= 1) return;

    let html = '';
    for (let i = 0; i < questions.length; i++) {
      const active = i === currentIdx ? 'active' : '';
      html += `<button class="q-nav-btn ${active}" data-idx="${i}">Q${i + 1}</button>`;
    }
    nav.innerHTML = html;

    // Attach events
    nav.querySelectorAll('.q-nav-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        currentIdx = parseInt(btn.dataset.idx, 10);
        renderQuestion(currentIdx);
      });
    });
  }

  function renderEmpty() {
    const container = $('#question-content');
    if (container) {
      container.innerHTML = '<p class="question-placeholder">No questions available.</p>';
    }
  }

  /** Very basic markdown → HTML (bold, italic, code, newlines) */
  function renderMarkdown(text) {
    if (!text) return '';
    let html = escapeHtml(text);
    // Code blocks (triple backtick)
    html = html.replace(/```([\s\S]*?)```/g, '<pre>$1</pre>');
    // Inline code
    html = html.replace(/`([^`]+)`/g, '<code>$1</code>');
    // Bold
    html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
    // Italic
    html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');
    // Line breaks
    html = html.replace(/\n/g, '<br>');
    return html;
  }

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str || '';
    return div.innerHTML;
  }

  return { init, loadQuestions, togglePanel, expandPanel };
})();
