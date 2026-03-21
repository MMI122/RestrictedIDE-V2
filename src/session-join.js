// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-join.js  (Join Session Form Logic)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const JoinSession = (() => {
  let heartbeatFailures = 0;
  let reconnectInterval = null;
  let reconnectInFlight = false;
  let disconnectCountdownInterval = null;
  let disconnectGraceRemaining = 0;
  const DEFAULT_DISCONNECT_GRACE_SECONDS = 120;
  let disconnectAutoSubmitTriggered = false;
  const seenBroadcastIds = new Set();

  function getDisconnectGraceSeconds() {
    const v = Number(Session.sessionData?.disconnectGraceSeconds);
    if (!Number.isFinite(v)) return DEFAULT_DISCONNECT_GRACE_SECONDS;
    return Math.max(15, Math.min(600, Math.floor(v)));
  }

  function isRemovedError(err) {
    const msg = String(err?.message || err || '').toLowerCase();
    return msg.includes('removed') || msg.includes('kicked') || msg.includes('forbidden');
  }

  function enterRemovedState() {
    if (Session.heartbeatInterval) {
      clearInterval(Session.heartbeatInterval);
      Session.heartbeatInterval = null;
    }
    stopReconnectLoop();
    stopDisconnectCountdown();
    reconnectInFlight = false;
    setConnectionState('disconnected', 'Removed');
    Session.showScreen('removed');
  }

  function init() {
    $('#join-session-form')?.addEventListener('submit', handleJoinSession);
    $('#btn-session-retry')?.addEventListener('click', () => {
      attemptReconnect(true);
    });

    // Auto-uppercase session code
    $('#join-code')?.addEventListener('input', (e) => {
      e.target.value = e.target.value.toUpperCase();
    });
  }

  function setConnectionState(state, label) {
    const indicator = $('#session-conn-indicator');
    const retryBtn = $('#btn-session-retry');
    const warningEl = $('#session-conn-warning');
    const countdownEl = $('#session-conn-countdown');
    if (!indicator) return;

    indicator.className = `session-conn-indicator ${state}`;
    indicator.textContent = label;

    if (retryBtn) {
      if (state === 'disconnected') {
        retryBtn.classList.remove('hidden');
      } else {
        retryBtn.classList.add('hidden');
      }
    }

    if (warningEl) {
      if (state === 'disconnected' || state === 'reconnecting') {
        warningEl.classList.remove('hidden');
        if (countdownEl) countdownEl.textContent = String(disconnectGraceRemaining);
      } else {
        warningEl.classList.add('hidden');
      }
    }
  }

  function stopDisconnectCountdown() {
    if (disconnectCountdownInterval) {
      clearInterval(disconnectCountdownInterval);
      disconnectCountdownInterval = null;
    }
  }

  function startDisconnectCountdown() {
    if (disconnectAutoSubmitTriggered) return;
    if (disconnectCountdownInterval) return;

    if (disconnectGraceRemaining <= 0) {
      disconnectGraceRemaining = getDisconnectGraceSeconds();
    }

    setConnectionState('disconnected', 'Disconnected');
    const countdownEl = $('#session-conn-countdown');
    if (countdownEl) countdownEl.textContent = String(disconnectGraceRemaining);

    disconnectCountdownInterval = setInterval(async () => {
      disconnectGraceRemaining -= 1;
      if (countdownEl) countdownEl.textContent = String(Math.max(disconnectGraceRemaining, 0));

      if (disconnectGraceRemaining <= 0) {
        stopDisconnectCountdown();
        disconnectAutoSubmitTriggered = true;
        appendOutput('error', '⚠ Connection lost too long. Auto-submitting and locking session.');
        try {
          await SubmitFlow.autoSubmit();
        } catch (err) {
          console.error('Forced auto-submit failed:', err);
        }
      }
    }, 1000);
  }

  async function sendHeartbeatOnce() {
    if (!Session.sessionData?.id) return;

    const isRemote = isRemoteServer(Session.sessionData.server);
    if (isRemote) {
      const heartbeatUrl = `http://${Session.sessionData.server}/api/session/${Session.sessionData.id}/heartbeat`;
      const res = await fetch(heartbeatUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          student_id: Session.sessionData.studentId,
        }),
      });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok || payload.ok === false) {
        const msg = payload.error || `HTTP ${res.status}`;
        throw new Error(msg);
      }
      return;
    }

    await invoke('heartbeat_cmd', {
      sessionId: Session.sessionData.id,
      studentId: Session.sessionData.studentId,
    });
  }

  async function pollBroadcasts() {
    if (!Session.sessionData?.id || !Session.sessionData?.studentId) return;

    const isRemote = isRemoteServer(Session.sessionData.server);
    let broadcasts = [];

    if (isRemote) {
      const url = `http://${Session.sessionData.server}/api/session/${Session.sessionData.id}/broadcasts/${Session.sessionData.studentId}`;
      const res = await fetch(url, { method: 'GET' });
      const payload = await res.json().catch(() => ({}));
      if (!res.ok || payload.ok === false) {
        throw new Error(payload.error || `HTTP ${res.status}`);
      }
      broadcasts = payload.data || [];
    } else {
      broadcasts = await invoke('get_student_broadcasts_cmd', {
        sessionId: Session.sessionData.id,
        studentId: Session.sessionData.studentId,
      });
    }

    for (const b of broadcasts) {
      if (!b?.id) continue;

      if (!seenBroadcastIds.has(b.id)) {
        seenBroadcastIds.add(b.id);
        appendOutput('info', `📢 [Broadcast] ${b.content}`);

        if (isRemote) {
          await fetch(`http://${Session.sessionData.server}/api/broadcast/${b.id}/delivered`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ student_id: Session.sessionData.studentId }),
          });
          await fetch(`http://${Session.sessionData.server}/api/broadcast/${b.id}/ack`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ student_id: Session.sessionData.studentId }),
          });
        } else {
          await invoke('mark_broadcast_delivered_cmd', {
            broadcastId: b.id,
            studentId: Session.sessionData.studentId,
          });
          await invoke('acknowledge_broadcast_cmd', {
            broadcastId: b.id,
            studentId: Session.sessionData.studentId,
          });
        }
      }
    }
  }

  function startReconnectLoop() {
    if (reconnectInterval) return;
    reconnectInterval = setInterval(() => {
      attemptReconnect(false);
    }, 5000);
  }

  function stopReconnectLoop() {
    if (reconnectInterval) {
      clearInterval(reconnectInterval);
      reconnectInterval = null;
    }
  }

  async function attemptReconnect(isManual) {
    if (reconnectInFlight || !Session.sessionData?.id) return;
    reconnectInFlight = true;
    try {
      setConnectionState('reconnecting', isManual ? 'Reconnecting...' : 'Auto-retrying...');

      const isRemote = isRemoteServer(Session.sessionData.server);
      if (isRemote) {
        await joinViaHttp(
          Session.sessionData.server,
          Session.sessionData.code,
          Session.sessionData.studentId,
          Session.sessionData.displayName || Session.sessionData.studentId,
        );
      } else {
        await invoke('join_session_cmd', {
          serverAddr: Session.sessionData.server,
          code: Session.sessionData.code,
          studentId: Session.sessionData.studentId,
          displayName: Session.sessionData.displayName,
        });
      }

      await sendHeartbeatOnce();
      heartbeatFailures = 0;
      stopReconnectLoop();
      stopDisconnectCountdown();
      disconnectGraceRemaining = getDisconnectGraceSeconds();
      setConnectionState('connected', 'Connected');
    } catch (err) {
      console.warn('Reconnect attempt failed:', err);
      if (isRemovedError(err)) {
        appendOutput('error', '⛔ You have been removed from this session by the administrator.');
        enterRemovedState();
        return;
      }
      setConnectionState('disconnected', 'Disconnected');
    } finally {
      reconnectInFlight = false;
    }
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

  function isRemoteServer(server) {
    // Check if server is not localhost/127.0.0.1 (i.e., it's a true remote LAN server)
    if (!server) return false;
    const addr = server.split(':')[0].toLowerCase();
    return addr !== 'localhost' && addr !== '127.0.0.1';
  }

  async function joinViaHttp(server, code, studentId, displayName) {
    // Join via HTTP request to remote LAN server
    const url = `http://${server}/api/session/${code}/join`;
    const payload = {
      student_id: studentId,
      display_name: displayName,
    };

    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });

    const payloadResp = await response.json().catch(() => ({}));

    if (!response.ok || payloadResp.ok === false) {
      const errorMsg = payloadResp.error || `HTTP ${response.status}`;
      throw new Error(`Join failed: ${errorMsg}`);
    }

    return payloadResp.data || payloadResp;
  }

  async function runSecurityChecks() {
    // Run pre-join security checks (VM, multi-monitor)
    try {
      // For local sessions, run native security checks
      const vmCheckResult = await invoke('check_vm').catch(e => {
        console.warn('VM check unavailable:', e);
        return { vm_detected: false };
      });

      if (vmCheckResult.is_vm) {
        const msg = 'VM detected: This application cannot run in a virtual machine.';
        showError(msg);
        return false;
      }

      const monitorCheckResult = await invoke('check_monitors').catch(e => {
        console.warn('Monitor check unavailable:', e);
        return { multi_monitor: false };
      });

      if ((monitorCheckResult.count || 0) > 1) {
        const msg = 'Multiple monitors detected: Exams must be conducted on a single monitor.';
        showError(msg);
        return false;
      }

      return true; // All checks passed
    } catch (err) {
      console.error('Security check error:', err);
      // Don't block the join on security check error, just log
      return true;
    }
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
    btn.textContent = 'Checking...';
    btn.disabled = true;

    try {
      showStatus('Running security checks...');
      
      // Run pre-join security checks (local only, skip for remote HTTP join)
      const isRemote = isRemoteServer(server);
      if (!isRemote) {
        const checksOk = await runSecurityChecks();
        if (!checksOk) {
          hideStatus();
          return; // Security check failed, error already shown
        }
      }

      btn.textContent = 'Connecting...';
      showStatus('Connecting to server...');

      let result;
      
      if (isRemote) {
        // Join via HTTP to remote LAN server
        showStatus('Joining remote session...');
        result = await joinViaHttp(server, code, studentId, displayName);
      } else {
        // Local IPC join (for development)
        result = await invoke('join_session_cmd', {
          serverAddr: server,
          code: code,
          studentId: studentId,
          displayName: displayName,
        });
      }

      showStatus('Joined successfully! Loading session...');

      // Store session data from join response
      seenBroadcastIds.clear();
      Session.sessionData = {
        id: result.session_id,
        code: code,
        name: result.name,
        duration: result.duration_minutes,
        remainingSeconds: result.remaining_seconds,
        questions: result.questions || [],
        allowedUrls: result.allowed_urls || [],
        disconnectGraceSeconds: result.options?.disconnect_grace_seconds || DEFAULT_DISCONNECT_GRACE_SECONDS,
        server: server,
        studentId: studentId,
        displayName: displayName,
        language: null,
      };
      Session.role = 'student';

      // Small delay for UX
      setTimeout(() => {
        // Activate kiosk lockdown on join
        activateKioskMode();

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
        disconnectAutoSubmitTriggered = false;
        disconnectGraceRemaining = getDisconnectGraceSeconds();
        setConnectionState('connected', 'Connected');
        startHeartbeat();
        pollBroadcasts().catch((e) => console.warn('Initial broadcast poll failed:', e));

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
    stopReconnectLoop();
    stopDisconnectCountdown();
    heartbeatFailures = 0;
    disconnectGraceRemaining = getDisconnectGraceSeconds();
    disconnectAutoSubmitTriggered = false;
    Session.heartbeatInterval = setInterval(async () => {
      try {
        await sendHeartbeatOnce();
        await pollBroadcasts();
        heartbeatFailures = 0;
        stopDisconnectCountdown();
        disconnectGraceRemaining = getDisconnectGraceSeconds();
        setConnectionState('connected', 'Connected');
      } catch (err) {
        console.warn('Heartbeat failed:', err);
        if (isRemovedError(err)) {
          appendOutput('error', '⛔ You have been removed from this session by the administrator.');
          enterRemovedState();
          return;
        }
        heartbeatFailures += 1;
        if (heartbeatFailures >= 3) {
          setConnectionState('disconnected', 'Disconnected');
          startDisconnectCountdown();
          startReconnectLoop();
        } else {
          setConnectionState('reconnecting', `Reconnecting (${heartbeatFailures}/3)`);
          startDisconnectCountdown();
        }
      }
    }, 15000);
  }

  function stopConnectionRecovery() {
    stopReconnectLoop();
    stopDisconnectCountdown();
    reconnectInFlight = false;
    heartbeatFailures = 0;
    disconnectGraceRemaining = getDisconnectGraceSeconds();
    disconnectAutoSubmitTriggered = false;
    setConnectionState('connected', 'Connected');
  }

  async function activateKioskMode() {
    // Activate kiosk lockdown on join (keyboard hooks, process monitoring, etc.)
    try {
      await invoke('set_kiosk_mode', { enabled: true }).catch(e => {
        console.warn('Kiosk activation warning:', e);
        // Non-critical: don't block if kiosk command fails
      });
    } catch (err) {
      console.error('Kiosk activation error:', err);
    }
  }

  return { init, startHeartbeat, stopConnectionRecovery };
})();
