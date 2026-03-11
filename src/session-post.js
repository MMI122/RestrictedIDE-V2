// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-post.js  (Post-Session Review — Admin)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const PostSession = (() => {
  let submissions = [];
  let selectedIndex = -1;

  function init() {
    $('#btn-run-all')?.addEventListener('click', handleRunAll);
    $('#btn-download-all')?.addEventListener('click', handleDownloadAll);
    $('#btn-export-csv')?.addEventListener('click', handleExportCsv);
    $('#btn-delete-session')?.addEventListener('click', handleDeleteSession);
    $('#btn-back-from-post')?.addEventListener('click', () => Session.showScreen('landing'));
  }

  async function load(sessionId, sessionName, sessionCode) {
    // Set header info
    $('#post-session-name').textContent = sessionName || 'Session Review';
    $('#post-session-code').textContent = sessionCode || '';

    // Reset
    submissions = [];
    selectedIndex = -1;
    $('#post-results-panel')?.classList.add('hidden');
    $('#post-viewer-title').textContent = 'Select a submission to view';
    $('#post-viewer-code').textContent = '';

    try {
      submissions = await invoke('get_session_submissions_cmd', { sessionId });
      // Filter to final submissions only
      submissions = submissions.filter(s => s.is_final);
    } catch (err) {
      console.error('Failed to load submissions:', err);
      submissions = [];
    }

    $('#post-submission-count').textContent = submissions.length;
    renderSidebar();
  }

  function renderSidebar() {
    const list = $('#post-submission-list');
    if (!list) return;

    if (submissions.length === 0) {
      list.innerHTML = '<div style="color:var(--text-secondary);font-size:12px;padding:16px;text-align:center;">No submissions yet.</div>';
      return;
    }

    list.innerHTML = submissions.map((s, i) => {
      const result = s.judge_result || 'pending';
      const active = i === selectedIndex ? ' active' : '';
      return `
        <div class="post-sub-item${active}" data-index="${i}" onclick="PostSession.selectSubmission(${i})">
          <div>
            <div class="sub-student">${escapeHtml(s.student_id)}</div>
            <div class="sub-file">${escapeHtml(s.filename)}</div>
          </div>
          <span class="sub-badge ${escapeHtml(result)}">${escapeHtml(result)}</span>
        </div>`;
    }).join('');
  }

  function selectSubmission(index) {
    if (index < 0 || index >= submissions.length) return;
    selectedIndex = index;
    const sub = submissions[index];

    // Update sidebar active state
    $$('.post-sub-item').forEach((el, i) => {
      el.classList.toggle('active', i === index);
    });

    // Show code
    const lang = sub.lang || guessLang(sub.filename);
    $('#post-viewer-title').textContent = `${sub.student_id} — ${sub.filename} (${lang})`;
    $('#post-viewer-code').textContent = sub.content || '';
  }

  // ── Run All ──

  async function handleRunAll() {
    const data = Session.sessionData;
    if (!data) return;

    const btn = $('#btn-run-all');
    if (btn) {
      btn.disabled = true;
      btn.textContent = '⏳ Judging...';
    }

    // Show overlay
    showRunningOverlay('Running batch evaluation...');

    try {
      const results = await invoke('judge_submissions_cmd', { sessionId: data.id });

      // Update local submissions with results
      for (const r of results) {
        const sub = submissions.find(s => s.id === r.submission_id);
        if (sub) {
          sub.judge_result = r.result;
          sub.judge_stdout = r.stdout;
          sub.judge_stderr = r.stderr;
          sub.exec_time_ms = r.exec_time_ms;
        }
      }

      renderSidebar();
      renderResultsTable(results);

      // Re-select if one was active
      if (selectedIndex >= 0) selectSubmission(selectedIndex);
    } catch (err) {
      console.error('Judge error:', err);
      alert('Batch evaluation failed: ' + (err.message || err));
    } finally {
      hideRunningOverlay();
      if (btn) {
        btn.disabled = false;
        btn.textContent = '▶ Run All';
      }
    }
  }

  function renderResultsTable(results) {
    const panel = $('#post-results-panel');
    const body = $('#post-results-body');
    if (!panel || !body) return;

    panel.classList.remove('hidden');
    body.innerHTML = results.map(r => {
      const badge = `<span class="sub-badge ${escapeHtml(r.result)}">${escapeHtml(r.result)}</span>`;
      return `<tr>
        <td>${escapeHtml(r.student_id)}</td>
        <td>${escapeHtml(r.filename)}</td>
        <td>${escapeHtml(r.lang || '')}</td>
        <td>${badge}</td>
        <td>${r.exec_time_ms != null ? r.exec_time_ms : '--'}</td>
      </tr>`;
    }).join('');
  }

  // ── Download All ──

  async function handleDownloadAll() {
    const data = Session.sessionData;
    if (!data) return;

    // Use user's Downloads folder as default
    const saveDir = await getDownloadsDir();

    const btn = $('#btn-download-all');
    if (btn) {
      btn.disabled = true;
      btn.textContent = '⏳ Creating zip...';
    }

    try {
      const zipPath = await invoke('download_submissions_zip_cmd', {
        sessionId: data.id,
        saveDir,
      });
      alert('Saved to: ' + zipPath);
    } catch (err) {
      console.error('Download error:', err);
      alert('Download failed: ' + (err.message || err));
    } finally {
      if (btn) {
        btn.disabled = false;
        btn.textContent = '⬇ Download All';
      }
    }
  }

  // ── Export CSV ──

  async function handleExportCsv() {
    const data = Session.sessionData;
    if (!data) return;

    try {
      const csv = await invoke('export_results_csv_cmd', { sessionId: data.id });
      // Download as file via blob
      const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `session-${data.code || data.id}-results.csv`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Export CSV error:', err);
      alert('Export failed: ' + (err.message || err));
    }
  }

  // ── Delete Session ──

  async function handleDeleteSession() {
    const data = Session.sessionData;
    if (!data) return;

    if (!confirm('⚠ DELETE this session permanently?\n\nAll submissions, participants, violations, and broadcasts will be lost.\n\nThis cannot be undone.')) {
      return;
    }

    // Second confirmation
    if (!confirm('Are you ABSOLUTELY sure? Type OK to confirm.')) {
      return;
    }

    showRunningOverlay('Deleting session...');

    try {
      await invoke('delete_session_cmd', { sessionId: data.id });
      hideRunningOverlay();
      alert('Session deleted.');
      Session.sessionData = null;
      Session.showScreen('landing');
    } catch (err) {
      hideRunningOverlay();
      console.error('Delete error:', err);
      alert('Delete failed: ' + (err.message || err));
    }
  }

  // ── Helpers ──

  function guessLang(filename) {
    const ext = filename.split('.').pop()?.toLowerCase();
    const map = { py: 'Python', js: 'JavaScript', c: 'C', cpp: 'C++', java: 'Java' };
    return map[ext] || ext || 'text';
  }

  function escapeHtml(str) {
    if (str == null) return '';
    const div = document.createElement('div');
    div.textContent = String(str);
    return div.innerHTML;
  }

  async function getDownloadsDir() {
    try {
      return await invoke('get_downloads_dir_cmd');
    } catch (_) { /* ignore */ }
    return 'C:\\Users\\Public\\Downloads';
  }

  function showRunningOverlay(msg) {
    // Remove existing
    hideRunningOverlay();
    const overlay = document.createElement('div');
    overlay.className = 'running-overlay';
    overlay.id = 'running-overlay';
    overlay.innerHTML = `<div class="running-overlay-inner"><div class="spinner"></div><div>${escapeHtml(msg)}</div></div>`;
    document.body.appendChild(overlay);
  }

  function hideRunningOverlay() {
    const el = document.getElementById('running-overlay');
    if (el) el.remove();
  }

  return { init, load, selectSubmission };
})();
