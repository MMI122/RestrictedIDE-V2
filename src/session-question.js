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
    $('#btn-question-toggle-top')?.addEventListener('click', togglePanel);
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
    const topBtn = $('#btn-question-toggle-top');
    if (!panel) return;

    panelCollapsed = true;
    panel.classList.add('collapsed');
    if (topBtn) {
      topBtn.textContent = '▶ Question';
      topBtn.title = 'Show question panel';
    }
  }

  function expandPanel() {
    const panel = $('#question-panel');
    const topBtn = $('#btn-question-toggle-top');
    if (!panel) return;

    panelCollapsed = false;
    panel.classList.remove('collapsed');
    if (topBtn) {
      topBtn.textContent = '◀ Question';
      topBtn.title = 'Hide question panel';
    }
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

    const visibleCases = Array.isArray(q.visible_testcases) ? q.visible_testcases : [];

    if (visibleCases.length > 0) {
      html += '<div class="question-section"><strong>Sample Testcases:</strong></div>';
      html += visibleCases.map((tc, i) => `
        <div class="question-section testcase-view">
          <div class="testcase-title">Case ${i + 1}</div>
          <div><strong>Input:</strong></div>
          <pre>${escapeHtml(tc.input || '')}</pre>
          <div><strong>Expected Output:</strong></div>
          <pre>${escapeHtml(tc.expected_output || '')}</pre>
        </div>
      `).join('');
    } else {
      const sampleInput = q.sample_input ?? q.input_data;
      if (sampleInput) {
        html += `
          <div class="question-section">
            <strong>Sample Input:</strong>
            <pre>${escapeHtml(sampleInput)}</pre>
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
    const frameEl = $('#doc-viewer-frame');
    if (!viewer || !titleEl || !urlEl || !loadingEl || !frameEl) {
      throw new Error('Docs viewer UI is not available');
    }

    viewer.classList.remove('hidden');
    loadingEl.classList.remove('hidden');
    frameEl.removeAttribute('src');
    frameEl.srcdoc = '<html><body style="font-family:sans-serif;padding:16px;">Loading document...</body></html>';
    titleEl.textContent = 'Loading...';
    urlEl.textContent = url;

    try {
      const data = await invoke('fetch_allowed_doc_cmd', {
        url,
        allowedUrls: allowedUrls || [],
      });

      titleEl.textContent = data?.title || 'Documentation';
      urlEl.textContent = data?.url || url;

      const html = String(data?.html || '').trim();
      const safeBase = escapeHtmlAttr(data?.url || url);
      frameEl.removeAttribute('src');
      frameEl.srcdoc = `<!doctype html><html><head><meta charset="utf-8"><base href="${safeBase}"></head><body>${html}</body></html>`;
      loadingEl.classList.add('hidden');
      return;
    } catch (err) {
      const msg = String(err?.message || err || '');
      const canFallback = msg.includes('HTTP 403') || msg.includes('Failed to fetch URL');
      if (!canFallback) {
        loadingEl.classList.add('hidden');
        throw err;
      }

      // Fallback for anti-bot/CSP sites : many block both fetch and iframe.; yeah a workaround ig. if anybody has better ideas lmk
      // Use external browser as a controlled workaround after allowlist validation.
      titleEl.textContent = 'External Browser Required';
      urlEl.textContent = url;
      frameEl.removeAttribute('src');
      frameEl.srcdoc = `
        <html>
          <body style="font-family:Segoe UI,sans-serif;padding:18px;background:#111;color:#ddd;line-height:1.5;">
            <h3 style="margin:0 0 10px 0;color:#fff;">Embedded view blocked by website</h3>
            <p style="margin:0 0 10px 0;">This allowed URL cannot be rendered inside the IDE viewer.</p>
            <p style="margin:0;word-break:break-all;color:#9ad;">${escapeHtml(url)}</p>
          </body>
        </html>
      `;
      loadingEl.classList.add('hidden');

      const shouldOpen = confirm('This site blocks in-app viewing. Open it in your default browser?');
      if (shouldOpen) {
        openExternalAllowedUrl(url);
      }
      appendOutput('info', `External-browser fallback available for: ${url}`);
    }
  }

  function openExternalAllowedUrl(url) {
    const opened = window.open(url, '_blank', 'noopener,noreferrer');
    if (!opened) {
      alert('Unable to open the external browser automatically. Please copy this URL:\n' + url);
    }
  }

  function closeDocViewer() {
    const viewer = $('#doc-viewer');
    const frameEl = $('#doc-viewer-frame');
    const loadingEl = $('#doc-viewer-loading');
    if (viewer) viewer.classList.add('hidden');
    if (frameEl) {
      frameEl.srcdoc = '';
      frameEl.removeAttribute('src');
    }
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

  function escapeHtmlAttr(str) {
    return String(str || '')
      .replace(/&/g, '&amp;')
      .replace(/"/g, '&quot;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  }

  return { init, loadQuestions, togglePanel, expandPanel };
})();
