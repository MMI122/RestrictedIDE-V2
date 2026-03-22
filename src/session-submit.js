// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – session-submit.js  (Submit Flow + Lock Screen)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const SubmitFlow = (() => {
  let submitted = false;

  const SUBMITTABLE_EXTENSIONS = new Set([
    '.c', '.cpp', '.h', '.hpp', '.py', '.java', '.js', '.ts', '.txt', '.md', '.json'
  ]);

  function init() {
    // Manual submit button
    $('#btn-submit-code')?.addEventListener('click', handleSubmit);
  }

  async function handleSubmit() {
    if (submitted) return;

    if (!confirm('Are you sure you want to submit your code? You will not be able to edit after submission.')) {
      return;
    }

    await doSubmit(false);
  }

  /** Called by CountdownTimer when time runs out */
  async function autoSubmit() {
    if (submitted) return;
    await doSubmit(true);
  }

  async function doSubmit(isAuto) {
    if (submitted) return;
    submitted = true;

    const data = Session.sessionData;
    if (!data) return;

    const filesToSubmit = await collectSubmissionFiles();
    if (filesToSubmit.length === 0) {
      filesToSubmit.push({
        filename: IDE.openTabs?.[IDE.activeTab]?.name || 'untitled.txt',
        content: $('#code-editor')?.value || '',
      });
    }

    try {
      // Check if remote server
      const isRemote = data.server && data.server.split(':')[0].toLowerCase() !== 'localhost' && data.server.split(':')[0].toLowerCase() !== '127.0.0.1';

      for (const file of filesToSubmit) {
        if (isRemote) {
          // Submit via HTTP to remote LAN server
          const submitUrl = `http://${data.server}/api/session/${data.id}/submit`;
          const response = await fetch(submitUrl, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              student_id: data.studentId,
              filename: file.filename,
              content: file.content,
              lang: data.language || guessLanguage(file.filename),
            }),
          });

          if (!response.ok) {
            const errorData = await response.json().catch(() => ({}));
            throw new Error(errorData.error || `HTTP ${response.status}`);
          }
        } else {
          // Local IPC submit
          await invoke('submit_code_cmd', {
            sessionId: data.id,
            studentId: data.studentId,
            filename: file.filename,
            content: file.content,
            lang: data.language || guessLanguage(file.filename),
          });
        }
      }

      // Stop timers
      CountdownTimer.stop();
      if (Session.heartbeatInterval) clearInterval(Session.heartbeatInterval);
      if (typeof JoinSession?.stopConnectionRecovery === 'function') {
        JoinSession.stopConnectionRecovery();
      }

      // Show lock screen
      showCompletionScreen(data, filesToSubmit, isAuto);

    } catch (err) {
      console.error('Submit error:', err);
      submitted = false; // allow retry
      if (isAuto) {
        appendOutput('error', '❌ Auto-submit failed: ' + (err.message || err));
        // Show notification
        alert('Auto-submit failed! Please submit manually.');
      } else {
        alert('Failed to submit: ' + (err.message || err));
      }
    }
  }

  function showCompletionScreen(data, submittedFiles, isAuto) {
    const primary = submittedFiles[0] || { filename: 'untitled.txt', content: '' };

    // Fill completion details
    $('#complete-student-id').textContent = data.studentId || '--';
    $('#complete-submit-time').textContent = new Date().toLocaleString();
    $('#complete-session-name').textContent = data.name || '--';
    $('#complete-code').textContent = primary.content;

    // Hide student session bar and question panel
    const studentBar = $('#student-session-bar');
    if (studentBar) studentBar.classList.add('hidden');
    const questionPanel = $('#question-panel');
    if (questionPanel) questionPanel.classList.add('hidden');

    // Hide IDE
    const ideElements = ['#toolbar', '#main-wrapper', '#output-bar', '#status-bar'];
    ideElements.forEach(sel => {
      const el = $(sel);
      if (el) el.style.display = 'none';
    });

    // Show completion screen
    Session.showScreen('complete');

    if (isAuto) {
      appendOutput('info', `⏰ Time is up! Auto-submitted ${submittedFiles.length} file(s).`);
    } else {
      appendOutput('info', `✅ Submitted ${submittedFiles.length} file(s) successfully.`);
    }
  }

  function getExt(name) {
    const dot = name.lastIndexOf('.');
    return dot >= 0 ? name.slice(dot).toLowerCase() : '';
  }

  function shouldSubmitFile(name) {
    return SUBMITTABLE_EXTENSIONS.has(getExt(name));
  }

  function toRelativeFilename(absPath) {
    if (!absPath) return 'untitled.txt';
    if (!IDE?.sandboxPath) return absPath;
    const prefix = IDE.sandboxPath.endsWith('\\') || IDE.sandboxPath.endsWith('/')
      ? IDE.sandboxPath
      : `${IDE.sandboxPath}\\`;
    const rel = absPath.startsWith(prefix) ? absPath.slice(prefix.length) : absPath;
    return rel.replace(/\\/g, '/');
  }

  async function collectPathsRecursive(dirPath, out) {
    const entries = await invoke('list_dir', { dirPath });
    for (const entry of entries) {
      if (entry.is_directory) {
        await collectPathsRecursive(entry.path, out);
      } else if (entry.is_file && shouldSubmitFile(entry.name)) {
        out.push(entry.path);
      }
    }
  }

  async function collectSubmissionFiles() {
    const filePaths = [];
    await collectPathsRecursive(IDE.sandboxPath, filePaths);

    const openTabContent = new Map();
    for (const tab of IDE.openTabs || []) {
      if (tab?.path) {
        openTabContent.set(tab.path, tab.content || '');
      }
    }

    const files = [];
    for (const path of filePaths) {
      const filename = toRelativeFilename(path);
      try {
        const content = openTabContent.has(path)
          ? openTabContent.get(path)
          : await invoke('read_file', { filePath: path });
        files.push({ filename, content });
      } catch (err) {
        console.warn('Skipping unreadable file during submit:', path, err);
      }
    }

    return files;
  }

  function guessLanguage(filename) {
    const ext = filename.split('.').pop()?.toLowerCase();
    const map = {
      'py': 'python',
      'js': 'javascript',
      'ts': 'typescript',
      'c': 'c',
      'cpp': 'cpp',
      'java': 'java',
    };
    return map[ext] || 'text';
  }

  return { init, autoSubmit, handleSubmit };
})();
