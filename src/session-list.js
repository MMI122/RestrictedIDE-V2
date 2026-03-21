// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-list.js  (Admin Session List)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const SessionList = (() => {
  function init() {
    $('#btn-refresh-session-list')?.addEventListener('click', () => load());
  }

  async function load() {
    const body = $('#session-list-body');
    if (!body) return;

    body.innerHTML = '<tr><td colspan="6" class="session-list-empty">Loading sessions...</td></tr>';

    try {
      const sessions = await invoke('list_sessions_cmd');
      renderRows(sessions || []);
    } catch (err) {
      console.error('Session list load error:', err);
      body.innerHTML = `<tr><td colspan="6" class="session-list-empty">Failed to load sessions: ${escapeHtml(err?.message || String(err))}</td></tr>`;
    }
  }

  function renderRows(sessions) {
    const body = $('#session-list-body');
    if (!body) return;

    if (!sessions.length) {
      body.innerHTML = '<tr><td colspan="6" class="session-list-empty">No sessions found.</td></tr>';
      return;
    }

    body.innerHTML = sessions.map((s) => {
      const status = (s.status || '').toLowerCase();
      const created = formatDate(s.created_at);
      return `
        <tr>
          <td>${escapeHtml(s.name || '--')}</td>
          <td><span class="mono">${escapeHtml(s.code || '--')}</span></td>
          <td><span class="status-badge ${escapeHtml(status)}">${escapeHtml(status || '--')}</span></td>
          <td>${Number(s.duration_minutes || 0)} min</td>
          <td>${escapeHtml(created)}</td>
          <td>
            <button class="landing-btn outline session-list-open-btn" onclick="SessionList.openReview('${escapeHtml(s.id)}')">Open</button>
          </td>
        </tr>`;
    }).join('');
  }

  async function openReview(sessionId) {
    try {
      const sessions = await invoke('list_sessions_cmd');
      const s = (sessions || []).find((row) => row.id === sessionId);
      if (!s) {
        alert('Session not found.');
        return;
      }

      Session.sessionData = {
        id: s.id,
        code: s.code,
        name: s.name,
        duration: s.duration_minutes,
      };
      Session.role = 'admin';

      Session.showScreen('post');
      await PostSession.load(s.id, s.name, s.code);
    } catch (err) {
      console.error('Open review error:', err);
      alert('Failed to open session review: ' + (err.message || err));
    }
  }

  function formatDate(value) {
    if (!value) return '--';
    const d = new Date(value);
    if (Number.isNaN(d.getTime())) return value;
    return d.toLocaleString();
  }

  function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = String(str ?? '');
    return div.innerHTML;
  }

  return { init, load, openReview };
})();
