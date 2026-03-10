// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-submit.js  (Submit Flow + Lock Screen)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const SubmitFlow = (() => {
  let submitted = false;

  function init() {
    // Manual submit button
    $('#btn-submit-code')?.addEventListener('click', handleSubmit);
  }

  async function handleSubmit() {
    if (submitted) return;

    if (!confirm('Are you sure you want to submit your code? You will not be able to edit after submission.')) {
      return;
    }

    await doSubmit(false);
  }

  /** Called by CountdownTimer when time runs out */
  async function autoSubmit() {
    if (submitted) return;
    await doSubmit(true);
  }

  async function doSubmit(isAuto) {
    if (submitted) return;
    submitted = true;

    const data = Session.sessionData;
    if (!data) return;

    // Get current code from editor
    const code = $('#code-editor')?.value || '';
    const filename = IDE.openTabs?.[IDE.activeTab]?.name || 'untitled.txt';

    try {
      await invoke('submit_code_cmd', {
        sessionId: data.id,
        participantId: data.participantId,
        questionId: '1',  // Currently single-question mode
        filename,
        code,
        language: data.language || guessLanguage(filename),
        isFinal: true,
      });

      // Stop timers
      CountdownTimer.stop();
      if (Session.heartbeatInterval) clearInterval(Session.heartbeatInterval);

      // Show lock screen
      showCompletionScreen(data, code, isAuto);

    } catch (err) {
      console.error('Submit error:', err);
      submitted = false; // allow retry
      if (isAuto) {
        appendOutput('error', '❌ Auto-submit failed: ' + (err.message || err));
        // Show notification
        alert('Auto-submit failed! Please submit manually.');
      } else {
        alert('Failed to submit: ' + (err.message || err));
      }
    }
  }

  function showCompletionScreen(data, code, isAuto) {
    // Fill completion details
    $('#complete-student-id').textContent = data.studentId || '--';
    $('#complete-submit-time').textContent = new Date().toLocaleString();
    $('#complete-session-name').textContent = data.name || '--';
    $('#complete-code').textContent = code;

    // Hide student session bar and question panel
    const studentBar = $('#student-session-bar');
    if (studentBar) studentBar.classList.add('hidden');
    const questionPanel = $('#question-panel');
    if (questionPanel) questionPanel.classList.add('hidden');

    // Hide IDE
    const ideElements = ['#toolbar', '#main-wrapper', '#output-bar', '#status-bar'];
    ideElements.forEach(sel => {
      const el = $(sel);
      if (el) el.style.display = 'none';
    });

    // Show completion screen
    Session.showScreen('complete');

    if (isAuto) {
      appendOutput('info', '⏰ Time is up! Your code has been auto-submitted.');
    } else {
      appendOutput('info', '✅ Code submitted successfully.');
    }
  }

  function guessLanguage(filename) {
    const ext = filename.split('.').pop()?.toLowerCase();
    const map = {
      'py': 'python',
      'js': 'javascript',
      'ts': 'typescript',
      'c': 'c',
      'cpp': 'cpp',
      'java': 'java',
    };
    return map[ext] || 'text';
  }

  return { init, autoSubmit, handleSubmit };
})();
