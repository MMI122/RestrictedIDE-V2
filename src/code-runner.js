// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – code-runner.js  (code execution, stdout/stderr streaming)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const CodeRunner = (() => {

  let isRunning = false;
  let unlisten = { output: null, exit: null };

  /** Run the currently active file. */
  async function run() {
    if (IDE.activeTab === null) {
      setStatus('No file open');
      return;
    }

    // Save first
    await Editor.save();

    const tab = IDE.openTabs[IDE.activeTab];

    // Clear output
    $('#output-console').innerHTML = '';
    $('#stdin-bar').classList.remove('hidden');
    isRunning = true;
    setStatus(`Running: ${tab.name}`);
    $('#btn-run').disabled = true;

    // Set up event listeners
    if (unlisten.output) { unlisten.output(); unlisten.output = null; }
    if (unlisten.exit) { unlisten.exit(); unlisten.exit = null; }

    unlisten.output = await listen('code-output', (event) => {
      const { type, text } = event.payload;
      appendOutput(type === 'stdout' ? 'stdout' : type === 'stderr' ? 'stderr' : 'info', text);
    });

    unlisten.exit = await listen('code-exit', (event) => {
      const { code, signal, error } = event.payload;
      isRunning = false;
      $('#btn-run').disabled = false;

      let msg = '\n';
      if (error) {
        msg += `⚠ ${error}\n`;
      }
      if (signal) {
        msg += `Process terminated by signal: ${signal}\n`;
      }
      if (code !== null && code !== undefined) {
        msg += `Process exited with code ${code}\n`;
      }
      appendOutput('info', msg);
      setStatus('Ready');
      $('#stdin-bar').classList.add('hidden');

      // Cleanup listeners
      if (unlisten.output) { unlisten.output(); unlisten.output = null; }
      if (unlisten.exit) { unlisten.exit(); unlisten.exit = null; }
    });

    // Invoke the run command
    try {
      console.log('[CodeRunner] Invoking run_code for:', tab.path);
      const result = await invoke('run_code', { filePath: tab.path });
      console.log('[CodeRunner] run_code result:', result);
      if (!result.running) {
        appendOutput('stderr', `Run failed: ${result.error || 'Unknown error'}\n`);
        isRunning = false;
        $('#btn-run').disabled = false;
        setStatus('Run failed');
      }
    } catch (e) {
      console.error('[CodeRunner] run_code error:', e);
      appendOutput('stderr', `Error: ${e}\n`);
      isRunning = false;
      $('#btn-run').disabled = false;
      setStatus('Run failed');
    }
  }

  /** Stop the running process. */
  async function stop() {
    if (!isRunning) return;
    try {
      await invoke('stop_code');
      appendOutput('info', '■ Process stopped.\n');
      isRunning = false;
      $('#btn-run').disabled = false;
      setStatus('Stopped');
    } catch (e) {
      appendOutput('stderr', `Stop error: ${e}\n`);
    }
  }

  /** Send stdin input. */
  async function sendInput() {
    const input = $('#stdin-input');
    const text = input.value;
    if (!text && text !== '') return;

    try {
      await invoke('send_code_input', { text });
      appendOutput('info', `> ${text}\n`);
      input.value = '';
    } catch (e) {
      appendOutput('stderr', `Input error: ${e}\n`);
    }
  }

  /* ── Wire buttons ───────────────────────────────────────────────────── */

  document.addEventListener('DOMContentLoaded', () => {
    $('#btn-run').addEventListener('click', run);
    $('#btn-stop').addEventListener('click', stop);
    $('#btn-send-input').addEventListener('click', sendInput);
    $('#stdin-input').addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        sendInput();
      }
    });
  });

  return { run, stop };
})();
