// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-join.js  (Join Session Form Logic)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const JoinSession = (() => {

  function init() {
    $('#join-session-form')?.addEventListener('submit', handleJoinSession);

    // Auto-uppercase session code
    $('#join-code')?.addEventListener('input', (e) => {
      e.target.value = e.target.value.toUpperCase();
    });
  }

  function showError(msg) {
    const el = $('#join-error');
    if (el) {
      el.textContent = msg;
      el.classList.remove('hidden');
    }
  }

  function hideError() {
    const el = $('#join-error');
    if (el) el.classList.add('hidden');
  }

  function showStatus(msg) {
    const el = $('#join-status');
    if (el) {
      el.textContent = msg;
      el.classList.remove('hidden');
    }
  }

  function hideStatus() {
    const el = $('#join-status');
    if (el) el.classList.add('hidden');
  }

  async function handleJoinSession(e) {
    e.preventDefault();
    hideError();
    hideStatus();

    const server = $('#join-server')?.value?.trim();
    const code = $('#join-code')?.value?.trim().toUpperCase();
    const studentId = $('#join-student-id')?.value?.trim();
    const displayName = $('#join-display-name')?.value?.trim();

    if (!server || !code || !studentId || !displayName) {
      showError('All fields are required.');
      return;
    }

    // Validate server format
    if (!server.includes(':')) {
      showError('Server address must include port (e.g. 192.168.1.100:9876)');
      return;
    }

    const btn = $('#btn-join-session');
    const originalText = btn.textContent;
    btn.textContent = 'Connecting...';
    btn.disabled = true;

    showStatus('Connecting to server...');

    try {
      const result = await invoke('join_session_cmd', {
        code: code,
        studentId: studentId,
      });

      showStatus('Joined successfully! Loading session...');

      // Store session data from join response
      Session.sessionData = {
        id: result.session_id,
        code: code,
        name: result.name,
        duration: result.duration_minutes,
        remainingSeconds: result.remaining_seconds,
        questions: result.questions || [],
        allowedUrls: result.allowed_urls || [],
        server: server,
        studentId: studentId,
        displayName: displayName,
        language: null,
      };
      Session.role = 'student';

      // Small delay for UX
      setTimeout(() => {
        // Enter student session mode (shows IDE + session bar + question panel)
        Session.enterStudentSession();

        // Load question content
        if (Session.sessionData.questions.length > 0) {
          QuestionPanel.loadQuestions(Session.sessionData.questions);
        }

        // Set session bar info
        const barName = $('#session-bar-name');
        if (barName) barName.textContent = Session.sessionData.name;

        // Start countdown timer — use remaining_seconds if session already started
        const secs = Session.sessionData.remainingSeconds || (Session.sessionData.duration * 60);
        CountdownTimer.start(secs);

        // Start heartbeat
        startHeartbeat();

      }, 500);

    } catch (err) {
      console.error('Join session error:', err);
      showError('Failed to join: ' + (err.message || err));
      hideStatus();
    } finally {
      btn.textContent = originalText;
      btn.disabled = false;
    }
  }

  function startHeartbeat() {
    // Send heartbeat every 15 seconds
    if (Session.heartbeatInterval) clearInterval(Session.heartbeatInterval);
    Session.heartbeatInterval = setInterval(async () => {
      try {
        if (Session.sessionData?.id) {
          await invoke('heartbeat_cmd', {
            sessionId: Session.sessionData.id,
            studentId: Session.sessionData.studentId,
          });
        }
      } catch (err) {
        console.warn('Heartbeat failed:', err);
      }
    }, 15000);
  }

  return { init, startHeartbeat };
})();
