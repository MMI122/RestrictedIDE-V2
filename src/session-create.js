// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-create.js  (Create Session Form Logic)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const CreateSession = (() => {
  function init() {
    // Add question button
    $('#btn-add-question')?.addEventListener('click', addQuestionCard);

    // Add URL button
    $('#btn-add-url')?.addEventListener('click', addUrlCard);

    // Form submission
    $('#create-session-form')?.addEventListener('submit', handleCreateSession);

    // Lockdown-specific settings visibility
    $('#sec-policy-mode')?.addEventListener('change', syncSecurityModeUI);
    syncSecurityModeUI();

    // Add first question by default
    addQuestionCard();
  }

  function isLockdownMode() {
    return ($('#sec-policy-mode')?.value || 'strict') === 'lockdown';
  }

  function syncSecurityModeUI() {
    const lockdownBox = $('#lockdown-options');
    const thresholdInput = $('#sec-focus-threshold');
    if (!lockdownBox || !thresholdInput) return;

    const lockdown = isLockdownMode();
    lockdownBox.classList.toggle('hidden', !lockdown);
    thresholdInput.disabled = lockdown;
    if (lockdown) {
      thresholdInput.value = '1';
    }
  }

  function addUrlCard() {
    const list = $('#urls-list');
    if (!list) return;

    Session.urlCount = (Session.urlCount || 0) + 1;
    const idx = Session.urlCount;

    const card = document.createElement('div');
    card.className = 'url-card';
    card.dataset.urlIdx = idx;
    card.innerHTML = `
      <div class="url-input-row">
        <input type="url" id="url-${idx}" placeholder="e.g. https://docs.python.org" />
        <button type="button" class="remove-url-btn" data-idx="${idx}">✕ Remove</button>
      </div>
    `;

    card.querySelector('.remove-url-btn').addEventListener('click', () => {
      card.remove();
    });

    list.appendChild(card);
  }

  function collectUrls() {
    const cards = document.querySelectorAll('.url-card');
    const urls = [];
    cards.forEach(card => {
      const idx = card.dataset.urlIdx;
      const url = card.querySelector(`#url-${idx}`)?.value?.trim();
      if (url) {
        urls.push(url);
      }
    });
    return urls;
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
        <label>Testcases</label>
        <div class="testcases-list" id="q-tests-${idx}"></div>
        <button type="button" class="add-testcase-btn" data-qidx="${idx}">+ Add Testcase</button>
      </div>
    `;

    // Remove button handler
    card.querySelector('.remove-question-btn').addEventListener('click', () => {
      card.remove();
      renumberQuestions();
    });

    card.querySelector('.add-testcase-btn').addEventListener('click', () => {
      addTestcaseRow(card, { hidden: false });
    });

    list.appendChild(card);
    addTestcaseRow(card, { hidden: false });
  }

  function addTestcaseRow(card, seed = {}) {
    const qidx = card.dataset.questionIdx;
    const list = card.querySelector(`#q-tests-${qidx}`);
    if (!list) return;

    const caseIdx = Number(card.dataset.caseCount || 0) + 1;
    card.dataset.caseCount = String(caseIdx);

    const row = document.createElement('div');
    row.className = 'testcase-row';
    row.dataset.caseIdx = String(caseIdx);
    row.innerHTML = `
      <div class="testcase-row-head">
        <span>Case ${caseIdx}</span>
        <button type="button" class="remove-testcase-btn">✕</button>
      </div>
      <div class="form-row">
        <label>Input</label>
        <textarea class="tc-input" rows="2" placeholder="Input for this testcase">${seed.input || ''}</textarea>
      </div>
      <div class="form-row">
        <label>Expected Output</label>
        <textarea class="tc-output" rows="2" placeholder="Expected output for this testcase">${seed.output || ''}</textarea>
      </div>
      <div class="form-row checkbox-row">
        <label><input type="checkbox" class="tc-hidden" ${seed.hidden ? 'checked' : ''} /> Hidden testcase</label>
      </div>
    `;

    row.querySelector('.remove-testcase-btn').addEventListener('click', () => {
      row.remove();
    });

    list.appendChild(row);
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
      const testcaseRows = Array.from(card.querySelectorAll('.testcase-row'));

      const visible_testcases = [];
      const hidden_testcases = [];

      testcaseRows.forEach((row) => {
        const input = row.querySelector('.tc-input')?.value ?? '';
        const expectedOutput = row.querySelector('.tc-output')?.value ?? '';
        const hidden = row.querySelector('.tc-hidden')?.checked ?? false;

        if (!input.trim() && !expectedOutput.trim()) return;

        const tc = {
          input,
          expected_output: expectedOutput,
          hidden,
        };

        if (hidden) {
          hidden_testcases.push(tc);
        } else {
          visible_testcases.push(tc);
        }
      });

      const firstVisible = visible_testcases[0] || null;

      questions.push({
        title,
        description,
        sample_input: firstVisible?.input || null,
        expected_output: firstVisible?.expected_output || null,
        visible_testcases,
        hidden_testcases,
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
    const disconnectGraceSeconds = parseInt($('#session-disconnect-grace')?.value, 10) || 120;
    const focusThreshold = parseInt($('#sec-focus-threshold')?.value, 10) || 3;
    const lockdownEmergencyUnlock = $('#sec-lockdown-emergency')?.checked ?? true;
    const lockdownPassword = ($('#sec-lockdown-password')?.value || '').trim();
    const lockdownPasswordConfirm = ($('#sec-lockdown-password-confirm')?.value || '').trim();

    if (!name) return;

    const questions = collectQuestions();
    if (questions.length === 0) {
      alert('Please add at least one question.');
      return;
    }

    // Collect security settings
    const security = {
      security_mode: $('#sec-policy-mode')?.value || 'strict',
      block_vm: $('#sec-vm-check')?.checked ?? true,
      block_multi_monitor: $('#sec-multi-monitor')?.checked ?? true,
      prevent_screenshots: $('#sec-screenshot')?.checked ?? true,
      focus_watchdog: $('#sec-focus-watch')?.checked ?? true,
      controlled_paste: $('#sec-controlled-paste')?.checked ?? true,
      focus_auto_submit_threshold: Math.max(1, Math.min(10, focusThreshold)),
      lockdown_emergency_unlock: lockdownEmergencyUnlock,
    };

    if (security.security_mode === 'lockdown') {
      if (!lockdownEmergencyUnlock) {
        alert('Lockdown mode requires Emergency Unlock to be enabled.');
        return;
      }

      if (!lockdownPassword || !lockdownPasswordConfirm) {
        alert('Enter and confirm the emergency unlock password for lockdown mode.');
        return;
      }

      if (lockdownPassword.length < 8) {
        alert('Emergency unlock password must be at least 8 characters.');
        return;
      }

      if (lockdownPassword !== lockdownPasswordConfirm) {
        alert('Emergency unlock password confirmation does not match.');
        return;
      }

      security.focus_auto_submit_threshold = 1;
    }

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
        visible_testcases: q.visible_testcases || [],
        hidden_testcases: q.hidden_testcases || [],
        time_limit_ms: null,
      }));

      // Collect allowed URLs
      const allowedUrls = collectUrls();

      const result = await invoke('create_session_cmd', {
        name,
        durationMinutes: duration,
        questions: questionInputs,
        allowedUrls,
        options: {
          video: false,
          audio: false,
          screen_share: false,
          recording: false,
          security_mode: security.security_mode,
          block_vm: security.block_vm,
          block_multi_monitor: security.block_multi_monitor,
          prevent_screenshots: security.prevent_screenshots,
          focus_watchdog: security.focus_watchdog,
          controlled_paste: security.controlled_paste,
          focus_auto_submit_threshold: security.focus_auto_submit_threshold,
          disconnect_grace_seconds: Math.max(15, Math.min(600, disconnectGraceSeconds)),
          lockdown_emergency_unlock: security.lockdown_emergency_unlock,
        },
        lockdownExitPassword: security.security_mode === 'lockdown' ? lockdownPassword : null,
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
