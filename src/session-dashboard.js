// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-dashboard.js  (Admin Session Dashboard)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const Dashboard = (() => {
  let pollTimer = null;
  let sessionActive = false;
  let selectedStudentId = null;
  let latestParticipants = [];
  let latestSubmissions = [];
  let latestViolations = [];
  let latestBroadcasts = [];
  let latestBroadcastReceipts = [];

  function init() {
    $('#btn-start-session')?.addEventListener('click', handleStartSession);
    $('#btn-end-session')?.addEventListener('click', handleEndSession);
    $('#btn-send-broadcast')?.addEventListener('click', () => handleBroadcast('all'));
    $('#btn-send-selected')?.addEventListener('click', () => handleBroadcast('selected'));
    $('#btn-review-submissions')?.addEventListener('click', handleReviewSubmissions);

    // Broadcast on Enter key
    $('#broadcast-input')?.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        handleBroadcast('all');
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
    $('#dash-submission-count').textContent = '0';
    $('#dash-violation-count').textContent = '0';
    $('#dash-latest-submission').textContent = '--';
    $('#dash-latest-violation').textContent = '--';
    $('#dash-last-refresh').textContent = '--';
    $('#dash-broadcast-count').textContent = '0';
    $('#dash-violation-feed-count').textContent = '0';
    latestBroadcasts = [];
    latestBroadcastReceipts = [];
    selectedStudentId = null;
    renderParticipantDetail();
    renderViolationFeed();
    renderBroadcastHistory();

    // Show start button, hide end button and review button
    const startBtn = $('#btn-start-session');
    const endBtn = $('#btn-end-session');
    const reviewBtn = $('#btn-review-submissions');
    if (startBtn) startBtn.style.display = '';
    if (endBtn) endBtn.style.display = 'none';
    if (reviewBtn) reviewBtn.style.display = 'none';

    // Start polling participants
    startPolling(data.id);
  }

  function startPolling(sessionId) {
    if (pollTimer) clearInterval(pollTimer);

    // Initial load
    refreshDashboardData(sessionId);

    // Poll every 3 seconds
    pollTimer = setInterval(() => {
      refreshDashboardData(sessionId);
    }, 3000);

    Session.pollInterval = pollTimer;
  }

  function stopPolling() {
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  }

  async function refreshDashboardData(sessionId) {
    try {
      const [statusResp, submissions, violations, broadcasts, receipts] = await Promise.all([
        invoke('get_session_status_cmd', { sessionId }),
        invoke('get_session_submissions_cmd', { sessionId }),
        invoke('get_session_violations_cmd', { sessionId }),
        invoke('get_session_broadcasts_cmd', { sessionId }),
        invoke('get_broadcast_receipts_cmd', { sessionId }),
      ]);

      const status = statusResp?.session?.status || 'created';
      latestParticipants = statusResp?.participants || [];
      latestSubmissions = submissions || [];
      latestViolations = violations || [];
      latestBroadcasts = broadcasts || [];
      latestBroadcastReceipts = receipts || [];

      updateSessionStatus(status);
      renderParticipants(latestParticipants);
      renderLiveMetrics(latestSubmissions, latestViolations);
      renderParticipantDetail();
      renderViolationFeed();
      renderBroadcastHistory();

      const refreshEl = $('#dash-last-refresh');
      if (refreshEl) refreshEl.textContent = new Date().toLocaleTimeString();
    } catch (err) {
      console.warn('Failed to refresh dashboard:', err);
    }
  }

  function updateSessionStatus(status) {
    const statusEl = $('#dashboard-session-status');
    if (!statusEl) return;

    const normalized = String(status || 'created').toLowerCase();
    const text = normalized.charAt(0).toUpperCase() + normalized.slice(1);
    statusEl.textContent = text;
    statusEl.className = `status-badge ${normalized}`;

    if (normalized === 'ended') {
      sessionActive = false;
      const endBtn = $('#btn-end-session');
      if (endBtn) endBtn.style.display = 'none';
      const reviewBtn = $('#btn-review-submissions');
      if (reviewBtn) reviewBtn.style.display = '';
      if (Session.timerInterval) {
        clearInterval(Session.timerInterval);
        Session.timerInterval = null;
      }
      stopPolling();
    }
  }

  function renderLiveMetrics(submissions, violations) {
    const subCountEl = $('#dash-submission-count');
    const vioCountEl = $('#dash-violation-count');
    const latestSubEl = $('#dash-latest-submission');
    const latestVioEl = $('#dash-latest-violation');

    if (subCountEl) subCountEl.textContent = String(submissions.length);
    if (vioCountEl) vioCountEl.textContent = String(violations.length);

    if (latestSubEl) {
      if (submissions.length === 0) {
        latestSubEl.textContent = '--';
      } else {
        const latest = [...submissions].sort((a, b) => new Date(b.submitted_at) - new Date(a.submitted_at))[0];
        latestSubEl.textContent = `${latest.student_id} (${latest.filename})`;
      }
    }

    if (latestVioEl) {
      if (violations.length === 0) {
        latestVioEl.textContent = '--';
      } else {
        const latest = [...violations].sort((a, b) => new Date(b.occurred_at) - new Date(a.occurred_at))[0];
        latestVioEl.textContent = `${latest.student_id} (${latest.event_type})`;
      }
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
      const selectedClass = selectedStudentId === p.student_id ? ' selected' : '';
      return `
        <div class="participant-row${selectedClass}" data-id="${p.id}" data-student-id="${escapeHtml(p.student_id)}">
          <div class="participant-info">
            <span class="participant-status-dot ${dotClass}"></span>
            <span class="participant-name">${escapeHtml(p.display_name || p.student_id)}</span>
            <span class="participant-id">${escapeHtml(p.student_id)}</span>
          </div>
          <div>
            <span class="status-badge ${dotClass}" style="font-size:10px;">${state}</span>
            ${state !== 'kicked'
              ? `<button class="kick-btn" onclick="Dashboard.kickParticipant('${escapeHtml(p.student_id)}')">Kick</button>`
              : `<button class="kick-btn permit" onclick="Dashboard.permitReentry('${escapeHtml(p.student_id)}')">Permit Re-entry</button>`}
          </div>
        </div>
      `;
    }).join('');

    list.querySelectorAll('.participant-row').forEach(row => {
      row.addEventListener('click', () => {
        selectedStudentId = row.dataset.studentId || null;
        renderParticipants(latestParticipants);
        renderParticipantDetail();
      });
    });

    if (selectedStudentId && !participants.some(p => p.student_id === selectedStudentId)) {
      selectedStudentId = null;
      renderParticipantDetail();
    }
  }

  function renderParticipantDetail() {
    const emptyEl = $('#participant-detail-empty');
    const contentEl = $('#participant-detail-content');

    const participant = latestParticipants.find(p => p.student_id === selectedStudentId);
    if (!participant) {
      if (emptyEl) emptyEl.classList.remove('hidden');
      if (contentEl) contentEl.classList.add('hidden');
      return;
    }

    if (emptyEl) emptyEl.classList.add('hidden');
    if (contentEl) contentEl.classList.remove('hidden');

    const submission = [...latestSubmissions]
      .filter(s => s.student_id === participant.student_id)
      .sort((a, b) => new Date(b.submitted_at) - new Date(a.submitted_at))[0];

    const participantViolations = latestViolations.filter(v => v.student_id === participant.student_id);
    const latestViolation = [...participantViolations]
      .sort((a, b) => new Date(b.occurred_at) - new Date(a.occurred_at))[0];

    $('#detail-display-name').textContent = participant.display_name || participant.student_id;
    $('#detail-student-id').textContent = participant.student_id;
    $('#detail-state').textContent = String(participant.state || 'joined');
    $('#detail-latest-submission').textContent = submission ? `${submission.filename} (${formatTime(submission.submitted_at)})` : '--';
    $('#detail-latest-violation').textContent = latestViolation ? `${latestViolation.event_type} (${formatTime(latestViolation.occurred_at)})` : '--';
    $('#detail-violation-count').textContent = String(participantViolations.length);
  }

  function renderViolationFeed() {
    const listEl = $('#violation-feed-list');
    const countEl = $('#dash-violation-feed-count');
    if (!listEl) return;

    const timeline = selectedStudentId
      ? latestViolations.filter(v => v.student_id === selectedStudentId)
      : latestViolations;

    if (countEl) countEl.textContent = String(timeline.length);

    if (timeline.length === 0) {
      listEl.innerHTML = '<div class="violation-feed-empty">No violations recorded yet.</div>';
      return;
    }

    listEl.innerHTML = timeline.slice(0, 40).map(v => {
      const sev = String(v.severity || 'warning').toLowerCase();
      return `
        <div class="violation-row ${sev}">
          <div class="violation-row-top">
            <span class="violation-student">${escapeHtml(v.student_id)}</span>
            <span class="violation-time">${formatTime(v.occurred_at)}</span>
          </div>
          <div class="violation-type">${escapeHtml(v.event_type || 'violation')}</div>
          ${v.details ? `<div class="violation-details">${escapeHtml(v.details)}</div>` : ''}
        </div>
      `;
    }).join('');
  }

  function formatTime(iso) {
    if (!iso) return '--';
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '--';
    return d.toLocaleTimeString();
  }

  function getParticipantDotClass(state) {
    switch (state) {
      case 'active':       return 'online';
      case 'joined':       return 'online';
      case 'submitted':    return 'submitted';
      case 'disconnected': return 'disconnected';
      case 'kicked':       return 'kicked';
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

      // Show review button
      const reviewBtn = $('#btn-review-submissions');
      if (reviewBtn) reviewBtn.style.display = '';

      // Stop timers
      if (Session.timerInterval) clearInterval(Session.timerInterval);
      stopPolling();

      setStatus('Session ended.');
    } catch (err) {
      console.error('End session error:', err);
      alert('Failed to end session: ' + (err.message || err));
    }
  }

  async function handleBroadcast(mode) {
    const input = $('#broadcast-input');
    const message = input?.value?.trim();
    if (!message || !Session.sessionData) return;

    const isSelected = mode === 'selected';
    if (isSelected && !selectedStudentId) {
      alert('Select a participant first, then use Send to Selected.');
      return;
    }

    const targetIds = isSelected ? [selectedStudentId] : null;

    try {
      await invoke('broadcast_message_cmd', {
        sessionId: Session.sessionData.id,
        content: message,
        targetType: isSelected ? 'specific' : 'all',
        targetIds,
      });

      await refreshDashboardData(Session.sessionData.id);

      input.value = '';
      appendOutput('info', isSelected
        ? `📢 [Broadcast -> ${selectedStudentId}] ${message}`
        : `📢 [Broadcast -> All] ${message}`);
    } catch (err) {
      console.error('Broadcast error:', err);
    }
  }

  function renderBroadcastHistory() {
    const listEl = $('#broadcast-history-list');
    const countEl = $('#dash-broadcast-count');
    if (!listEl) return;

    if (countEl) countEl.textContent = String(latestBroadcasts.length);

    if (latestBroadcasts.length === 0) {
      listEl.innerHTML = '<div class="broadcast-history-empty">No broadcasts sent yet.</div>';
      return;
    }

    listEl.innerHTML = latestBroadcasts.map(item => {
      const states = getRecipientStates(item);
      const summary = summarizeRecipientStates(states);
      const chips = states.slice(0, 6).map(s => `
        <span class="recipient-chip ${s.stateClass}">${escapeHtml(s.studentId)}: ${s.stateLabel}</span>
      `).join('');
      const more = states.length > 6 ? `<span class="recipient-chip more">+${states.length - 6} more</span>` : '';

      return `
        <div class="broadcast-history-item">
          <div class="broadcast-history-meta">
            <span class="broadcast-time">${formatTime(item.created_at)}</span>
            <span class="broadcast-summary">${summary}</span>
          </div>
          <div class="broadcast-history-content">${escapeHtml(item.content)}</div>
          <div class="broadcast-recipient-chips">${chips}${more}</div>
        </div>
      `;
    }).join('');
  }

  function getRecipientStates(item) {
    const participantsById = new Map((latestParticipants || []).map(p => [p.student_id, p]));
    const receipts = latestBroadcastReceipts.filter(r => r.broadcast_id === item.id);

    return receipts.map(r => {
      const studentId = r.student_id;
      const p = participantsById.get(studentId);
      const state = String(p?.state || '').toLowerCase();

      if (r.acknowledged_at) {
        return { studentId, stateLabel: 'ack', stateClass: 'ack' };
      }
      if (r.delivered_at) {
        return { studentId, stateLabel: 'delivered', stateClass: 'delivered' };
      }
      if (state === 'kicked') {
        return { studentId, stateLabel: 'unreachable', stateClass: 'unreachable' };
      }
      return { studentId, stateLabel: 'pending', stateClass: 'pending' };
    });
  }

  function summarizeRecipientStates(states) {
    const ack = states.filter(s => s.stateClass === 'ack').length;
    const delivered = states.filter(s => s.stateClass === 'delivered').length;
    const pending = states.filter(s => s.stateClass === 'pending').length;
    const unreachable = states.filter(s => s.stateClass === 'unreachable').length;

    return `ack ${ack} | delivered ${delivered} | pending ${pending} | unreachable ${unreachable}`;
  }

  async function kickParticipant(participantId) {
    if (!Session.sessionData) return;
    if (!confirm('Kick this participant?')) return;

    try {
      await invoke('kick_participant_cmd', {
        sessionId: Session.sessionData.id,
        studentId: participantId,
      });
      // Will be reflected on next poll
    } catch (err) {
      console.error('Kick error:', err);
      alert('Failed to kick participant: ' + (err.message || err));
    }
  }

  async function permitReentry(participantId) {
    if (!Session.sessionData) return;

    try {
      await invoke('permit_reentry_cmd', {
        sessionId: Session.sessionData.id,
        studentId: participantId,
      });
      appendOutput('info', `✅ Permit re-entry granted for ${participantId}`);
    } catch (err) {
      console.error('Permit re-entry error:', err);
      alert('Failed to permit re-entry: ' + (err.message || err));
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

  function handleReviewSubmissions() {
    const data = Session.sessionData;
    if (!data) return;
    stopPolling();
    Session.showScreen('post');
    PostSession.load(data.id, data.name, data.code);
  }

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
  }

  return {
    init,
    load,
    kickParticipant,
    permitReentry,
    stopPolling,
    get sessionActive() { return sessionActive; },
  };
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
