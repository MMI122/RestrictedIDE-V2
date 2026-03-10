// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-question.js  (Question Panel for Students)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const QuestionPanel = (() => {
  let questions = [];
  let currentIdx = 0;
  let panelVisible = true;

  function init() {
    // Toggle panel
    $('#btn-toggle-question')?.addEventListener('click', togglePanel);
  }

  function togglePanel() {
    const panel = $('#question-panel');
    const btn = $('#btn-toggle-question');
    if (!panel) return;

    panelVisible = !panelVisible;
    if (panelVisible) {
      panel.classList.remove('hidden');
      if (btn) btn.textContent = '◀';
    } else {
      panel.classList.add('hidden');
      if (btn) btn.textContent = '▶';
    }
  }

  function loadQuestions(questionList) {
    questions = questionList || [];
    currentIdx = 0;

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

  return { init, loadQuestions, togglePanel };
})();
