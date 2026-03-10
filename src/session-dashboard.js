// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-dashboard.js  (Admin Session Dashboard)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const Dashboard = (() => {
  let pollTimer = null;
  let sessionActive = false;

  function init() {
    $('#btn-start-session')?.addEventListener('click', handleStartSession);
    $('#btn-end-session')?.addEventListener('click', handleEndSession);
    $('#btn-send-broadcast')?.addEventListener('click', handleBroadcast);

    // Broadcast on Enter key
    $('#broadcast-input')?.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        handleBroadcast();
      }
    });
  }

  function load(data) {
    // Set header info
    $('#dashboard-session-name').textContent = data.name;
    $('#dashboard-session-code').textContent = data.code;
    $('#dashboard-session-status').textContent = 'Created';
    $('#dashboard-session-status').className = 'status-badge created';

    // Connection info
    $('#dash-lan-address').textContent = data.lan_address || `localhost:${data.port}`;
    $('#dash-session-code-display').textContent = data.code;

    // Reset timer
    $('#dashboard-timer').textContent = `${data.duration}:00`;

    // Show start button, hide end button
    const startBtn = $('#btn-start-session');
    const endBtn = $('#btn-end-session');
    if (startBtn) startBtn.style.display = '';
    if (endBtn) endBtn.style.display = 'none';

    // Start polling participants
    startPolling(data.id);
  }

  function startPolling(sessionId) {
    if (pollTimer) clearInterval(pollTimer);

    // Initial load
    refreshParticipants(sessionId);

    // Poll every 3 seconds
    pollTimer = setInterval(() => {
      refreshParticipants(sessionId);
    }, 3000);

    Session.pollInterval = pollTimer;
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  async function refreshParticipants(sessionId) {
    try {
      const participants = await invoke('get_session_participants_cmd', {
        sessionId,
      });
      renderParticipants(participants);
    } catch (err) {
      console.warn('Failed to refresh participants:', err);
    }
  }

  function renderParticipants(participants) {
    const list = $('#participant-list');
    const count = $('#dash-participant-count');
    if (!list) return;

    if (count) count.textContent = participants.length;

    if (participants.length === 0) {
      list.innerHTML = '<div style="color:var(--text-secondary);font-size:12px;padding:12px;text-align:center;">Waiting for participants to join...</div>';
      return;
    }

    list.innerHTML = participants.map(p => {
      const state = p.state || 'Joined';
      const dotClass = getParticipantDotClass(state);
      return `
        <div class="participant-row" data-id="${p.id}">
          <div class="participant-info">
            <span class="participant-status-dot ${dotClass}"></span>
            <span class="participant-name">${escapeHtml(p.display_name)}</span>
            <span class="participant-id">${escapeHtml(p.student_id)}</span>
          </div>
          <div>
            <span class="status-badge ${dotClass}" style="font-size:10px;">${state}</span>
            ${state !== 'Kicked' ? `<button class="kick-btn" onclick="Dashboard.kickParticipant('${p.id}')">Kick</button>` : ''}
          </div>
        </div>
      `;
    }).join('');
  }

  function getParticipantDotClass(state) {
    switch (state) {
      case 'Active':       return 'online';
      case 'Joined':       return 'online';
      case 'Submitted':    return 'submitted';
      case 'Disconnected': return 'disconnected';
      case 'Kicked':       return 'kicked';
      default:             return 'online';
    }
  }

  async function handleStartSession() {
    const data = Session.sessionData;
    if (!data) return;

    try {
      await invoke('start_session_cmd', { sessionId: data.id });

      sessionActive = true;

      // Update UI
      $('#dashboard-session-status').textContent = 'Active';
      $('#dashboard-session-status').className = 'status-badge active';

      const startBtn = $('#btn-start-session');
      const endBtn = $('#btn-end-session');
      if (startBtn) startBtn.style.display = 'none';
      if (endBtn) endBtn.style.display = '';

      // Start admin-side countdown timer
      startAdminTimer(data.duration * 60);

      setStatus('Session started!');
    } catch (err) {
      console.error('Start session error:', err);
      alert('Failed to start session: ' + (err.message || err));
    }
  }

  async function handleEndSession() {
    const data = Session.sessionData;
    if (!data) return;

    if (!confirm('Are you sure you want to end this session? All participants will be locked out.')) {
      return;
    }

    try {
      await invoke('end_session_cmd', { sessionId: data.id });

      sessionActive = false;

      // Update UI
      $('#dashboard-session-status').textContent = 'Ended';
      $('#dashboard-session-status').className = 'status-badge ended';

      const endBtn = $('#btn-end-session');
      if (endBtn) endBtn.style.display = 'none';

      // Stop timers
      if (Session.timerInterval) clearInterval(Session.timerInterval);
      stopPolling();

      setStatus('Session ended.');
    } catch (err) {
      console.error('End session error:', err);
      alert('Failed to end session: ' + (err.message || err));
    }
  }

  async function handleBroadcast() {
    const input = $('#broadcast-input');
    const message = input?.value?.trim();
    if (!message || !Session.sessionData) return;

    try {
      await invoke('broadcast_message_cmd', {
        sessionId: Session.sessionData.id,
        message,
        target: 'All',
        targetIds: [],
      });

      input.value = '';
      appendOutput('info', `📢 [Broadcast] ${message}`);
    } catch (err) {
      console.error('Broadcast error:', err);
    }
  }

  async function kickParticipant(participantId) {
    if (!Session.sessionData) return;
    if (!confirm('Kick this participant?')) return;

    try {
      await invoke('kick_participant_cmd', {
        sessionId: Session.sessionData.id,
        participantId,
      });
      // Will be reflected on next poll
    } catch (err) {
      console.error('Kick error:', err);
      alert('Failed to kick participant: ' + (err.message || err));
    }
  }

  function startAdminTimer(totalSeconds) {
    let remaining = totalSeconds;

    if (Session.timerInterval) clearInterval(Session.timerInterval);

    Session.timerInterval = setInterval(() => {
      remaining--;
      if (remaining <= 0) {
        clearInterval(Session.timerInterval);
        $('#dashboard-timer').textContent = '00:00';
        // Auto-end session
        handleEndSession();
        return;
      }

      const mins = Math.floor(remaining / 60);
      const secs = remaining % 60;
      $('#dashboard-timer').textContent =
        `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
    }, 1000);
  }

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  return { init, load, kickParticipant, stopPolling };
})();

// Handle back from dashboard
function handleBackFromDashboard() {
  if (Dashboard.sessionActive) {
    if (!confirm('Session is active. Going back will not stop the session. Continue?')) {
      return;
    }
  }
  Dashboard.stopPolling();
  if (Session.timerInterval) clearInterval(Session.timerInterval);
  Session.showScreen('landing');
}
