// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-post.js  (Post-Session Review — Admin)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const PostSession = (() => {
  let submissions = [];
  let violations = [];
  let participantStateByStudent = new Map();
  let participantNameByStudent = new Map();
  let selectedIndex = -1;
  let selectedViolationStudent = null;
  let activeStudentFilter = 'all';
  let lastJudgeResults = [];
  let focusMode = 'none';

  function init() {
    $('#btn-run-all')?.addEventListener('click', handleRunAll);
    $('#btn-download-all')?.addEventListener('click', handleDownloadAll);
    $('#btn-export-csv')?.addEventListener('click', handleExportCsv);
    $('#btn-delete-session')?.addEventListener('click', handleDeleteSession);
    $('#btn-focus-submissions')?.addEventListener('click', () => toggleFocus('submissions'));
    $('#btn-focus-violations-list')?.addEventListener('click', () => toggleFocus('violations-list'));
    $('#btn-focus-judge')?.addEventListener('click', () => toggleFocus('judge'));
    $('#btn-focus-violation-timeline')?.addEventListener('click', () => toggleFocus('violation-timeline'));
    $('#btn-focus-code')?.addEventListener('click', () => toggleFocus('code'));
    $('#post-student-filter')?.addEventListener('change', (e) => {
      setActiveStudentFilter(e.target.value);
    });
    setupResizers();
    $('#btn-back-from-post')?.addEventListener('click', () => {
      Session.showScreen('sessionList');
      SessionList.load();
    });
  }

  async function load(sessionId, sessionName, sessionCode) {
    // Set header info
    $('#post-session-name').textContent = sessionName || 'Session Review';
    $('#post-session-code').textContent = sessionCode || '';

    // Reset
    submissions = [];
    violations = [];
    selectedIndex = -1;
    selectedViolationStudent = null;
    activeStudentFilter = 'all';
    lastJudgeResults = [];
    setFocus('none');
    $('#post-results-panel')?.classList.add('hidden');
    $('#post-viewer-title').textContent = 'Select a submission to view';
    $('#post-viewer-code').textContent = '';

    try {
      submissions = await invoke('get_session_submissions_cmd', { sessionId });
      // Filter to final submissions only
      submissions = submissions.filter(s => s.is_final);

      const participants = await invoke('get_session_participants_cmd', { sessionId });
      participantStateByStudent = new Map(
        (participants || []).map((p) => [p.student_id, (p.state || 'joined').toLowerCase()]),
      );
      participantNameByStudent = new Map(
        (participants || []).map((p) => [p.student_id, p.display_name || p.student_id]),
      );

      violations = await invoke('get_session_violations_cmd', { sessionId });
      violations = (violations || []).sort((a, b) => new Date(b.occurred_at) - new Date(a.occurred_at));

    } catch (err) {
      console.error('Failed to load submissions:', err);
      submissions = [];
      violations = [];
      participantStateByStudent = new Map();
      participantNameByStudent = new Map();
    }

    submissions.sort((a, b) => {
      if (a.student_id !== b.student_id) return String(a.student_id).localeCompare(String(b.student_id));
      return new Date(b.submitted_at) - new Date(a.submitted_at);
    });

    populateStudentFilter();
    setActiveStudentFilter('all');
  }

  function populateStudentFilter() {
    const select = $('#post-student-filter');
    if (!select) return;

    const ids = new Set();
    participantNameByStudent.forEach((_, sid) => ids.add(sid));
    submissions.forEach((s) => ids.add(s.student_id));
    violations.forEach((v) => ids.add(v.student_id));

    const sortedIds = Array.from(ids).sort((a, b) => String(a).localeCompare(String(b)));
    const options = ['<option value="all">All students</option>'];
    for (const sid of sortedIds) {
      const name = participantNameByStudent.get(sid) || sid;
      options.push(`<option value="${escapeHtml(sid)}">${escapeHtml(name)} (${escapeHtml(sid)})</option>`);
    }
    select.innerHTML = options.join('');

    if (activeStudentFilter !== 'all' && !ids.has(activeStudentFilter)) {
      activeStudentFilter = 'all';
    }
    select.value = activeStudentFilter;
  }

  function setActiveStudentFilter(studentId) {
    activeStudentFilter = studentId || 'all';
    const select = $('#post-student-filter');
    if (select && select.value !== activeStudentFilter) {
      select.value = activeStudentFilter;
    }

    if (activeStudentFilter === 'all') {
      selectedViolationStudent = violations[0]?.student_id || null;
    } else {
      selectedViolationStudent = activeStudentFilter;
    }

    if (selectedIndex >= 0) {
      const selected = submissions[selectedIndex];
      if (!selected || !isStudentVisible(selected.student_id)) {
        selectedIndex = -1;
        $('#post-viewer-title').textContent = 'Select a submission to view';
        $('#post-viewer-code').textContent = '';
      }
    }

    renderSidebar();
    renderViolationSummary();
    renderViolationTimeline();

    if (lastJudgeResults.length > 0) {
      renderResultsTable(lastJudgeResults);
    }
  }

  function isStudentVisible(studentId) {
    return activeStudentFilter === 'all' || String(studentId) === String(activeStudentFilter);
  }

  function getFilteredSubmissions() {
    return submissions.filter((s) => isStudentVisible(s.student_id));
  }

  function getFilteredViolations() {
    return violations.filter((v) => isStudentVisible(v.student_id));
  }

  function renderSidebar() {
    const list = $('#post-submission-list');
    if (!list) return;

    const filteredSubmissions = getFilteredSubmissions();
    $('#post-submission-count').textContent = String(filteredSubmissions.length);

    if (filteredSubmissions.length === 0) {
      list.innerHTML = '<div style="color:var(--text-secondary);font-size:12px;padding:16px;text-align:center;">No submissions yet.</div>';
      return;
    }

    const grouped = new Map();
    submissions.forEach((s, i) => {
      if (!isStudentVisible(s.student_id)) return;
      const key = String(s.student_id || 'unknown');
      if (!grouped.has(key)) grouped.set(key, []);
      grouped.get(key).push({ sub: s, idx: i });
    });

    let html = '';
    for (const [studentId, rows] of grouped.entries()) {
      html += `
        <div class="post-student-group">
          <div class="post-student-head">${escapeHtml(studentId)} <span class="post-student-count">${rows.length}</span></div>
      `;
      html += rows.map(({ sub, idx }) => {
        const result = (sub.judge_result || 'pending').toLowerCase();
        const state = getDisplayStatus(sub);
        const active = idx === selectedIndex ? ' active' : '';
        return `
          <div class="post-sub-item${active}" data-index="${idx}">
            <div>
              <div class="sub-file">${escapeHtml(sub.filename)} • ${escapeHtml(state)}</div>
            </div>
            <span class="sub-badge ${escapeHtml(result)}">${escapeHtml(result)}</span>
          </div>
        `;
      }).join('');
      html += '</div>';
    }

    list.innerHTML = html;
    list.querySelectorAll('.post-sub-item').forEach((el) => {
      el.addEventListener('click', () => {
        const idx = Number(el.dataset.index);
        if (Number.isFinite(idx)) selectSubmission(idx);
      });
    });
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

    if (!selectedViolationStudent) {
      selectedViolationStudent = sub.student_id;
      renderViolationSummary();
      renderViolationTimeline();
    }
  }

  function renderViolationSummary() {
    const list = $('#post-violation-student-list');
    if (!list) return;

    const scopedViolations = getFilteredViolations();
    const total = scopedViolations.length;
    const critical = scopedViolations.filter((v) => String(v.severity || '').toLowerCase() === 'critical').length;

    const grouped = new Map();
    for (const v of scopedViolations) {
      const sid = v.student_id || 'unknown';
      if (!grouped.has(sid)) {
        grouped.set(sid, { total: 0, critical: 0, warning: 0, latest: null });
      }
      const row = grouped.get(sid);
      row.total += 1;
      const sev = String(v.severity || '').toLowerCase();
      if (sev === 'critical') row.critical += 1;
      else row.warning += 1;
      if (!row.latest || new Date(v.occurred_at) > new Date(row.latest.occurred_at)) {
        row.latest = v;
      }
    }

    $('#post-violation-total').textContent = String(total);
    $('#post-violation-students').textContent = String(grouped.size);
    $('#post-violation-critical').textContent = String(critical);

    if (grouped.size === 0) {
      list.innerHTML = '<div class="post-violation-empty">No violations recorded.</div>';
      return;
    }

    const rows = Array.from(grouped.entries())
      .sort((a, b) => b[1].total - a[1].total || String(a[0]).localeCompare(String(b[0])));

    list.innerHTML = rows.map(([studentId, info]) => {
      const active = selectedViolationStudent === studentId ? ' active' : '';
      const name = participantNameByStudent.get(studentId) || studentId;
      const latestText = info.latest ? `${escapeHtml(info.latest.event_type)} • ${formatTime(info.latest.occurred_at)}` : '--';
      return `
        <div class="post-violation-student${active}" data-student-id="${escapeHtml(studentId)}">
          <div class="post-violation-student-top">
            <span class="post-violation-name">${escapeHtml(name)}</span>
            <span class="post-violation-count">${info.total}</span>
          </div>
          <div class="post-violation-student-sub">${escapeHtml(studentId)}</div>
          <div class="post-violation-metrics">C:${info.critical} W:${info.warning}</div>
          <div class="post-violation-latest">${latestText}</div>
        </div>
      `;
    }).join('');

    list.querySelectorAll('.post-violation-student').forEach((el) => {
      el.addEventListener('click', () => {
        selectedViolationStudent = el.dataset.studentId || null;
        renderViolationSummary();
        renderViolationTimeline();
      });
    });
  }

  function renderViolationTimeline() {
    const timeline = $('#post-violation-timeline');
    const selectedLabel = $('#post-violation-selected');
    if (!timeline || !selectedLabel) return;

    const scopedViolations = getFilteredViolations();
    let filtered = scopedViolations;
    if (activeStudentFilter === 'all' && selectedViolationStudent) {
      filtered = scopedViolations.filter((v) => v.student_id === selectedViolationStudent);
      const name = participantNameByStudent.get(selectedViolationStudent) || selectedViolationStudent;
      selectedLabel.textContent = `${name} (${selectedViolationStudent})`;
    } else if (activeStudentFilter !== 'all') {
      const name = participantNameByStudent.get(activeStudentFilter) || activeStudentFilter;
      selectedLabel.textContent = `${name} (${activeStudentFilter})`;
    } else {
      selectedLabel.textContent = 'All students';
    }

    if (filtered.length === 0) {
      timeline.innerHTML = '<div class="post-violation-empty">No timeline items.</div>';
      return;
    }

    timeline.innerHTML = filtered.slice(0, 120).map((v) => {
      const sev = String(v.severity || 'warning').toLowerCase();
      return `
        <div class="post-violation-row ${escapeHtml(sev)}">
          <div class="post-violation-row-top">
            <span class="post-violation-row-student">${escapeHtml(v.student_id)}</span>
            <span class="post-violation-row-time">${formatTime(v.occurred_at)}</span>
          </div>
          <div class="post-violation-row-type">${escapeHtml(v.event_type || 'violation')}</div>
          ${v.details ? `<div class="post-violation-row-details">${escapeHtml(v.details)}</div>` : ''}
        </div>
      `;
    }).join('');
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
      lastJudgeResults = Array.isArray(results) ? results : [];

      // Update local submissions with results
      for (const r of lastJudgeResults) {
        const sub = submissions.find(s => s.id === r.submission_id);
        if (sub) {
          sub.judge_result = r.result;
          sub.judge_stdout = r.stdout;
          sub.judge_stderr = r.stderr;
          sub.exec_time_ms = r.exec_time_ms;
        }
      }

      renderSidebar();
      renderResultsTable(lastJudgeResults);

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

    const scopedResults = (results || []).filter((r) => isStudentVisible(r.student_id));
    panel.classList.remove('hidden');
    body.innerHTML = scopedResults.map(r => {
      const state = getDisplayStatus(r);
      const badge = `<span class="sub-badge ${escapeHtml(r.result)}">${escapeHtml(r.result)}</span>`;
      const caseHint = (r.failed_case_index && r.total_cases)
        ? `<div class="judge-case-hint">Failed at case ${r.failed_case_index}/${r.total_cases}</div>`
        : (r.total_cases ? `<div class="judge-case-hint">Passed ${r.total_cases}/${r.total_cases} cases</div>` : '');
      return `<tr>
        <td>${escapeHtml(r.student_id)}</td>
        <td>${escapeHtml(state)}</td>
        <td>${escapeHtml(r.filename)}</td>
        <td>${escapeHtml(r.question_hint || '--')}</td>
        <td>${escapeHtml(r.lang || '')}</td>
        <td>${badge}${caseHint}</td>
        <td>${r.exec_time_ms != null ? r.exec_time_ms : '--'}</td>
      </tr>`;
    }).join('');
  }

  // ── Download All ──

  async function handleDownloadAll() {
    const data = Session.sessionData;
    if (!data) return;

    // Ask teacher where to save zip; prefill with Downloads
    const defaultDir = await getDownloadsDir();
    const saveDir = window.prompt('Enter folder path to save submissions ZIP:', defaultDir);
    if (!saveDir || !saveDir.trim()) return;

    const btn = $('#btn-download-all');
    if (btn) {
      btn.disabled = true;
      btn.textContent = '⏳ Creating zip...';
    }

    try {
      const zipPath = await invoke('download_submissions_zip_cmd', {
        sessionId: data.id,
        saveDir: saveDir.trim(),
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

    const typed = window.prompt('Type DELETE to permanently remove this session:');
    if (typed !== 'DELETE') {
      return;
    }

    showRunningOverlay('Deleting session...');

    try {
      await invoke('delete_session_cmd', { sessionId: data.id });
      hideRunningOverlay();
      alert('Session deleted.');
      Session.sessionData = null;
      Session.showScreen('sessionList');
      await SessionList.load();
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

  function getDisplayStatus(item) {
    const result = (item.judge_result || item.result || 'pending').toLowerCase();
    if (result === 'timeout') return 'timed-out';
    if (result === 'compile_error') return 'compile-error';
    if (result !== 'pending') return 'judged';
    return participantStateByStudent.get(item.student_id) || 'submitted';
  }

  function formatTime(iso) {
    if (!iso) return '--';
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return '--';
    return d.toLocaleTimeString();
  }

  function toggleFocus(mode) {
    setFocus(focusMode === mode ? 'none' : mode);
  }

  function setFocus(mode) {
    focusMode = mode;
    const container = $('#post-session-container');
    if (!container) return;

    container.classList.remove('focus-submissions', 'focus-violations-list', 'focus-judge', 'focus-violation-timeline', 'focus-code');
    if (mode !== 'none') {
      container.classList.add(`focus-${mode}`);
    }

    const activeMap = {
      submissions: '#btn-focus-submissions',
      'violations-list': '#btn-focus-violations-list',
      judge: '#btn-focus-judge',
      'violation-timeline': '#btn-focus-violation-timeline',
      code: '#btn-focus-code',
    };

    Object.values(activeMap).forEach((sel) => {
      const btn = $(sel);
      if (btn) btn.classList.remove('active');
    });

    if (mode !== 'none') {
      const btn = $(activeMap[mode]);
      if (btn) btn.classList.add('active');
    }
  }

  function setupResizers() {
    const container = $('#post-session-container');
    const main = container?.querySelector('.post-main');
    const sidebar = container?.querySelector('.post-sidebar');
    const sidebarHandle = $('#post-sidebar-resize-handle');
    const splitResults = $('#post-splitter-results');
    const splitViolation = $('#post-splitter-violation');
    const resultsPanel = $('#post-results-panel');
    const violationPanel = $('#post-violation-panel');
    const codePanel = $('#post-code-viewer');

    if (!container || !main || !sidebar || !resultsPanel || !violationPanel || !codePanel) return;

    if (sidebarHandle && !sidebarHandle.dataset.bound) {
      sidebarHandle.dataset.bound = '1';
      sidebarHandle.addEventListener('pointerdown', (e) => {
        if (focusMode === 'submissions' || focusMode === 'violations-list') return;
        e.preventDefault();
        const rect = container.getBoundingClientRect();
        const startX = e.clientX;
        const startW = sidebar.getBoundingClientRect().width;

        const onMove = (ev) => {
          const dx = ev.clientX - startX;
          const minW = 220;
          const maxW = Math.max(420, rect.width * 0.65);
          const next = Math.min(maxW, Math.max(minW, startW + dx));
          container.style.setProperty('--post-sidebar-w', `${next}px`);
        };

        const onUp = () => {
          window.removeEventListener('pointermove', onMove);
          window.removeEventListener('pointerup', onUp);
          document.body.style.userSelect = '';
        };

        document.body.style.userSelect = 'none';
        window.addEventListener('pointermove', onMove);
        window.addEventListener('pointerup', onUp);
      });
    }

    if (splitResults && !splitResults.dataset.bound) {
      splitResults.dataset.bound = '1';
      splitResults.addEventListener('pointerdown', (e) => {
        if (focusMode !== 'none') return;
        if (getComputedStyle(resultsPanel).display === 'none') return;
        e.preventDefault();

        const startY = e.clientY;
        const mainH = main.getBoundingClientRect().height;
        const startResults = resultsPanel.getBoundingClientRect().height;
        const violationNow = violationPanel.getBoundingClientRect().height;
        const codeMin = 150;
        const violationMin = 110;
        const splitterSpace = 12;

        const onMove = (ev) => {
          const dy = ev.clientY - startY;
          const minH = 120;
          const maxH = Math.max(minH, mainH - violationNow - codeMin - splitterSpace);
          const next = Math.min(maxH, Math.max(minH, startResults + dy));
          container.style.setProperty('--post-results-h', `${next}px`);
        };

        const onUp = () => {
          window.removeEventListener('pointermove', onMove);
          window.removeEventListener('pointerup', onUp);
          document.body.style.userSelect = '';
        };

        document.body.style.userSelect = 'none';
        window.addEventListener('pointermove', onMove);
        window.addEventListener('pointerup', onUp);
      });
    }

    if (splitViolation && !splitViolation.dataset.bound) {
      splitViolation.dataset.bound = '1';
      splitViolation.addEventListener('pointerdown', (e) => {
        if (focusMode !== 'none') return;
        if (getComputedStyle(violationPanel).display === 'none') return;
        e.preventDefault();

        const startY = e.clientY;
        const mainH = main.getBoundingClientRect().height;
        const resultsNow = resultsPanel.getBoundingClientRect().height;
        const startViolation = violationPanel.getBoundingClientRect().height;
        const codeMin = 150;
        const splitterSpace = 12;

        const onMove = (ev) => {
          const dy = ev.clientY - startY;
          const minH = 100;
          const maxH = Math.max(minH, mainH - resultsNow - codeMin - splitterSpace);
          const next = Math.min(maxH, Math.max(minH, startViolation + dy));
          container.style.setProperty('--post-violation-h', `${next}px`);
        };

        const onUp = () => {
          window.removeEventListener('pointermove', onMove);
          window.removeEventListener('pointerup', onUp);
          document.body.style.userSelect = '';
        };

        document.body.style.userSelect = 'none';
        window.addEventListener('pointermove', onMove);
        window.addEventListener('pointerup', onUp);
      });
    }
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
