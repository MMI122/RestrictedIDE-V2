// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – admin.js  (admin login, exit, session management)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const Admin = (() => {

  /** Show the admin login dialog. */
  function showDialog() {
    const overlay = $('#admin-overlay');
    overlay.classList.remove('hidden');
    $('#admin-password').value = '';
    $('#admin-error').textContent = '';
    $('#admin-panel').classList.add('hidden');
    $('#admin-password').focus();

    // Check if already logged in
    checkSession();
  }

  function hideDialog() {
    $('#admin-overlay').classList.add('hidden');
  }

  /** Check current session. */
  async function checkSession() {
    try {
      const active = await invoke('admin_check_session');
      if (active) {
        $('#admin-password').style.display = 'none';
        $('#btn-admin-login').style.display = 'none';
        $('#admin-panel').classList.remove('hidden');
      }
    } catch (_) { /* ignore */ }
  }

  /** Login with password. */
  async function login() {
    const password = $('#admin-password').value;
    if (!password) {
      $('#admin-error').textContent = 'Enter a password';
      return;
    }

    try {
      const result = await invoke('admin_login', { password });
      if (result.success) {
        $('#admin-error').textContent = '';
        $('#admin-password').style.display = 'none';
        $('#btn-admin-login').style.display = 'none';
        $('#admin-panel').classList.remove('hidden');
        setStatus('Admin authenticated');
      } else {
        let msg = result.error || 'Login failed';
        if (result.attempts_remaining > 0) {
          msg += ` (${result.attempts_remaining} attempt(s) left)`;
        }
        $('#admin-error').textContent = msg;
      }
    } catch (e) {
      $('#admin-error').textContent = 'Error: ' + e;
    }
  }

  /** Logout. */
  async function logout() {
    try {
      await invoke('admin_logout');
      $('#admin-password').style.display = '';
      $('#btn-admin-login').style.display = '';
      $('#admin-panel').classList.add('hidden');
      setStatus('Admin logged out');
    } catch (e) {
      console.error('Logout error:', e);
    }
  }

  /** Exit the application (admin only). */
  async function exitApp() {
    try {
      await invoke('admin_request_exit');
    } catch (e) {
      $('#admin-error').textContent = 'Exit failed: ' + e;
    }
  }

  /* ── Wire buttons ───────────────────────────────────────────────────── */

  document.addEventListener('DOMContentLoaded', () => {
    $('#btn-admin-login').addEventListener('click', login);
    $('#btn-admin-cancel').addEventListener('click', hideDialog);
    $('#btn-admin-logout').addEventListener('click', logout);
    $('#btn-admin-exit').addEventListener('click', exitApp);

    $('#admin-password').addEventListener('keydown', (e) => {
      if (e.key === 'Enter') login();
      if (e.key === 'Escape') hideDialog();
    });
  });

  return { showDialog };
})();
