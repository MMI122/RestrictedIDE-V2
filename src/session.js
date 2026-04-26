// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session.js  (Session UI Management)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const Session = (() => {
  const EMERGENCY_RESTORE_PASSWORD = 'hello@123##wewouldrockit!';

  // ── Screen references ──
  const screens = {
    landing:    () => $('#landing-screen'),
    sessionList:() => $('#session-list-screen'),
    create:     () => $('#create-session-screen'),
    join:       () => $('#join-session-screen'),
    dashboard:  () => $('#admin-dashboard-screen'),
    post:       () => $('#post-session-screen'),
    complete:   () => $('#session-complete-screen'),
    removed:    () => $('#session-removed-screen'),
  };

  // ── State ──
  let currentScreen = 'landing';
  let sessionData = null;        // current session info
  let role = 'none';             // 'admin' | 'student' | 'none'
  let timerInterval = null;
  let heartbeatInterval = null;
  let pollInterval = null;
  let questionCount = 0;

  // ── Navigation ──

  function showScreen(name) {
    // Hide all screens
    Object.values(screens).forEach(fn => {
      const el = fn();
      if (el) el.classList.add('hidden');
    });

    // Hide IDE elements when not in practice or student-active mode
    const ideElements = ['#toolbar', '#main-wrapper', '#output-bar', '#status-bar'];
    const studentBar = $('#student-session-bar');
    const questionPanel = $('#question-panel');

    if (name === 'landing' || name === 'sessionList' || name === 'create' || name === 'join' || name === 'dashboard' || name === 'post' || name === 'complete' || name === 'removed') {
      ideElements.forEach(sel => {
        const el = $(sel);
        if (el) el.style.display = 'none';
      });
      if (studentBar) studentBar.classList.add('hidden');
      if (questionPanel) questionPanel.classList.add('hidden');
    }

    // Show requested screen
    const target = screens[name];
    if (target) {
      const el = target();
      if (el) el.classList.remove('hidden');
    }

    currentScreen = name;
  }

  function showIDE() {
    // Hide all session screens
    Object.values(screens).forEach(fn => {
      const el = fn();
      if (el) el.classList.add('hidden');
    });

    // Show IDE
    const ideElements = ['#toolbar', '#main-wrapper', '#output-bar', '#status-bar'];
    ideElements.forEach(sel => {
      const el = $(sel);
      if (el) el.style.display = '';
    });
  }

  function enterStudentSession() {
    showIDE();
    const studentBar = $('#student-session-bar');
    if (studentBar) studentBar.classList.remove('hidden');
    const questionPanel = $('#question-panel');
    if (questionPanel) {
      questionPanel.classList.remove('hidden');
      questionPanel.classList.remove('collapsed');
    }
    const questionOpenBtn = $('#btn-open-question-panel');
    if (questionOpenBtn) questionOpenBtn.classList.add('hidden');

    const barStatus = $('#session-bar-status');
    if (barStatus) {
      barStatus.textContent = 'Active';
      barStatus.className = 'status-badge active';
    }
  }

  // ── Initialization ──

  function init() {
    // Landing screen buttons
    $('#btn-goto-create')?.addEventListener('click', () => showScreen('create'));
    $('#btn-goto-join')?.addEventListener('click', () => showScreen('join'));
    $('#btn-goto-session-list')?.addEventListener('click', () => {
      showScreen('sessionList');
      SessionList.load();
    });
    $('#btn-emergency-restore-shell')?.addEventListener('click', requestEmergencyShellRestore);
    $('#btn-goto-practice')?.addEventListener('click', () => {
      showIDE();
      currentScreen = 'ide';
    });

    // Back buttons
    $('#btn-back-from-create')?.addEventListener('click', () => showScreen('landing'));
    $('#btn-back-from-join')?.addEventListener('click', () => showScreen('landing'));
    $('#btn-back-from-session-list')?.addEventListener('click', () => showScreen('landing'));
    $('#btn-back-from-dashboard')?.addEventListener('click', handleBackFromDashboard);

    // Show landing on start
    showScreen('landing');

    // Initialize sub-modules
    CreateSession.init();
    JoinSession.init();
    Dashboard.init();
    QuestionPanel.init();
    SubmitFlow.init();
    SessionList.init();
    PostSession.init();
  }

  async function requestEmergencyShellRestore() {
    const entered = window.prompt('Enter emergency restore password:');
    if (entered === null) return;

    if (entered !== EMERGENCY_RESTORE_PASSWORD) {
      alert('Incorrect password. Emergency restore denied.');
      return;
    }

    const proceed = window.confirm('This will disable exam-shell restrictions for RestrictedExam and restore normal shell. Continue?');
    if (!proceed) return;

    try {
      const result = await invoke('emergency_restore_shell_cmd', {
        password: entered,
        examUser: 'RestrictedExam',
      });

      if (result?.success) {
        alert((result?.message || 'Emergency restore completed.') + '\n\nPlease restart or sign out/in for full effect.');
      } else {
        alert('Emergency restore failed: ' + (result?.message || 'Unknown error'));
      }
    } catch (err) {
      console.error('Emergency restore shell failed:', err);
      alert('Emergency restore failed: ' + (err?.message || err));
    }
  }

  return { init, showScreen, showIDE, enterStudentSession, get currentScreen() { return currentScreen; }, get sessionData() { return sessionData; }, set sessionData(v) { sessionData = v; }, get role() { return role; }, set role(v) { role = v; }, get timerInterval() { return timerInterval; }, set timerInterval(v) { timerInterval = v; }, get heartbeatInterval() { return heartbeatInterval; }, set heartbeatInterval(v) { heartbeatInterval = v; }, get pollInterval() { return pollInterval; }, set pollInterval(v) { pollInterval = v; }, get questionCount() { return questionCount; }, set questionCount(v) { questionCount = v; } };
})();
