// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-create.js  (Create Session Form Logic)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const CreateSession = (() => {
  function init() {
    // Add question button
    $('#btn-add-question')?.addEventListener('click', addQuestionCard);

    // Form submission
    $('#create-session-form')?.addEventListener('submit', handleCreateSession);

    // Add first question by default
    addQuestionCard();
  }

  function addQuestionCard() {
    const list = $('#questions-list');
    if (!list) return;

    Session.questionCount++;
    const idx = Session.questionCount;

    const card = document.createElement('div');
    card.className = 'question-card';
    card.dataset.questionIdx = idx;
    card.innerHTML = `
      <div class="question-card-header">
        <span>Question ${idx}</span>
        <button type="button" class="remove-question-btn" data-idx="${idx}">✕ Remove</button>
      </div>
      <div class="form-row">
        <label for="q-title-${idx}">Title</label>
        <input type="text" id="q-title-${idx}" placeholder="e.g. Fibonacci Sequence" required />
      </div>
      <div class="form-row">
        <label for="q-desc-${idx}">Description</label>
        <textarea id="q-desc-${idx}" placeholder="Write the problem statement..." rows="4" required></textarea>
      </div>
      <div class="form-row">
        <label for="q-input-${idx}">Sample Input (optional)</label>
        <textarea id="q-input-${idx}" placeholder="e.g. 5" rows="2"></textarea>
      </div>
      <div class="form-row">
        <label for="q-output-${idx}">Expected Output (optional)</label>
        <textarea id="q-output-${idx}" placeholder="e.g. 0 1 1 2 3" rows="2"></textarea>
      </div>
    `;

    // Remove button handler
    card.querySelector('.remove-question-btn').addEventListener('click', () => {
      card.remove();
      renumberQuestions();
    });

    list.appendChild(card);
  }

  function renumberQuestions() {
    const cards = document.querySelectorAll('.question-card');
    cards.forEach((card, i) => {
      const header = card.querySelector('.question-card-header span');
      if (header) header.textContent = `Question ${i + 1}`;
    });
  }

  function collectQuestions() {
    const cards = document.querySelectorAll('.question-card');
    const questions = [];
    cards.forEach((card, i) => {
      const idx = card.dataset.questionIdx;
      const title = card.querySelector(`#q-title-${idx}`)?.value?.trim() || `Question ${i + 1}`;
      const description = card.querySelector(`#q-desc-${idx}`)?.value?.trim() || '';
      const sampleInput = card.querySelector(`#q-input-${idx}`)?.value?.trim() || '';
      const expectedOutput = card.querySelector(`#q-output-${idx}`)?.value?.trim() || '';

      questions.push({
        title,
        description,
        sample_input: sampleInput || null,
        expected_output: expectedOutput || null,
      });
    });
    return questions;
  }

  async function handleCreateSession(e) {
    e.preventDefault();

    const name = $('#session-name')?.value?.trim();
    const duration = parseInt($('#session-duration')?.value, 10);
    const language = $('#session-language')?.value;
    const mode = $('#session-mode')?.value;
    const port = parseInt($('#session-port')?.value, 10) || 9876;

    if (!name) return;

    const questions = collectQuestions();
    if (questions.length === 0) {
      alert('Please add at least one question.');
      return;
    }

    // Collect security settings
    const security = {
      block_vm: $('#sec-vm-check')?.checked ?? true,
      block_multi_monitor: $('#sec-multi-monitor')?.checked ?? true,
      prevent_screenshots: $('#sec-screenshot')?.checked ?? true,
      focus_watchdog: $('#sec-focus-watch')?.checked ?? true,
    };

    const btn = $('#btn-create-session');
    const originalText = btn.textContent;
    btn.textContent = 'Creating...';
    btn.disabled = true;

    try {
      // Map questions to QuestionInput format expected by Rust
      const questionInputs = questions.map(q => ({
        title: q.title,
        description: q.description,
        input_data: q.sample_input || null,
        expected_output: q.expected_output || null,
        time_limit_ms: null,
      }));

      const result = await invoke('create_session_cmd', {
        name,
        durationMinutes: duration,
        questions: questionInputs,
        allowedUrls: [],
        options: {
          video: false,
          audio: false,
          screen_share: false,
          recording: false,
        },
      });

      // Store session data
      Session.sessionData = {
        id: result.session_id,
        code: result.code,
        name,
        duration,
        language,
        mode,
        port,
        questions,
        security,
        lan_address: result.server_addr,
      };
      Session.role = 'admin';

      // Navigate to dashboard
      Dashboard.load(Session.sessionData);
      Session.showScreen('dashboard');

      appendOutput('info', `✅ Session created: ${result.code} at ${result.server_addr}`);

    } catch (err) {
      console.error('Create session error:', err);
      alert('Failed to create session: ' + (err.message || err));
    } finally {
      btn.textContent = originalText;
      btn.disabled = false;
    }
  }

  return { init };
})();
