// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-timer.js  (Countdown Timer)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const CountdownTimer = (() => {
  let remaining = 0;
  let interval = null;

  /**
   * Start countdown from totalSeconds.
   * Updates both the student session bar timer and triggers auto-submit at 0.
   */
  function start(totalSeconds) {
    remaining = totalSeconds;
    stop(); // clear any existing timer

    updateDisplay();

    interval = setInterval(() => {
      remaining--;

      if (remaining <= 0) {
        remaining = 0;
        stop();
        updateDisplay();
        onTimeUp();
        return;
      }

      updateDisplay();
      updateTimerStyle();
    }, 1000);

    Session.timerInterval = interval;
  }

  function stop() {
    if (interval) {
      clearInterval(interval);
      interval = null;
    }
  }

  function updateDisplay() {
    const mins = Math.floor(remaining / 60);
    const secs = remaining % 60;
    const timeStr = `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;

    // Student session bar timer
    const barTimer = $('#session-bar-timer');
    if (barTimer) barTimer.textContent = timeStr;
  }

  function updateTimerStyle() {
    const barTimer = $('#session-bar-timer');
    if (!barTimer) return;

    // Remove previous classes
    barTimer.classList.remove('warning', 'critical');

    if (remaining <= 60) {
      // Last minute — critical (red blinking)
      barTimer.classList.add('critical');
    } else if (remaining <= 300) {
      // Last 5 minutes — warning (yellow)
      barTimer.classList.add('warning');
    }
  }

  function onTimeUp() {
    // Auto-submit
    SubmitFlow.autoSubmit();
  }

  function getRemaining() {
    return remaining;
  }

  return { start, stop, getRemaining };
})();
