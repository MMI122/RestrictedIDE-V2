// ═══════════════════════════════════════════════════════════════════════════
// Restricted IDE – file-tree.js  (file explorer)
// ═══════════════════════════════════════════════════════════════════════════

'use strict';

const FileTree = (() => {

  const treeEl = () => $('#file-tree');

  /** Load a directory into the file tree. */
  async function load(dirPath) {
    try {
      const entries = await invoke('list_dir', { dirPath });
      IDE.currentPath = dirPath;
      render(entries);
      updateBreadcrumbs();
    } catch (e) {
      console.error('FileTree.load error:', e);
      setStatus('Cannot open: ' + e);
    }
  }

  /** Render the flat entry list. */
  function render(entries) {
    const tree = treeEl();
    tree.innerHTML = '';

    // Back button (if not at sandbox root)
    if (IDE.currentPath !== IDE.sandboxPath) {
      const back = document.createElement('div');
      back.className = 'tree-item';
      back.innerHTML = '<span class="icon">⬆️</span><span>..</span>';
      back.addEventListener('click', () => {
        const parent = IDE.currentPath.replace(/[/\\][^/\\]+$/, '');
        if (parent && parent.length >= IDE.sandboxPath.length) {
          load(parent);
        }
      });
      tree.appendChild(back);
    }

    entries.forEach(entry => {
      const item = document.createElement('div');
      item.className = 'tree-item';
      item.title = entry.path;

      const icon = document.createElement('span');
      icon.className = 'icon';
      icon.textContent = fileIcon(entry.name, entry.is_directory);
      item.appendChild(icon);

      const label = document.createElement('span');
      label.textContent = entry.name;
      item.appendChild(label);

      if (entry.is_directory) {
        item.addEventListener('click', () => load(entry.path));
      } else {
        item.addEventListener('click', () => openFileFromTree(entry));
      }

      // Context menu: rename / delete / copy path
      item.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        e.stopPropagation();
        showContextMenu(e.clientX, e.clientY, entry);
      });

      tree.appendChild(item);
    });
  }

  /** Open a file from the tree. */
  async function openFileFromTree(entry) {
    try {
      setStatus(`Opening ${entry.name}…`);
      const content = await invoke('read_file', { filePath: entry.path });
      Editor.openFile(entry.path, entry.name, content);
      setStatus(`Opened: ${entry.name}`);

      // Mark active in tree
      $$('.tree-item').forEach(el => el.classList.remove('active'));
      // Find the matching one
      $$('.tree-item').forEach(el => {
        if (el.title === entry.path) el.classList.add('active');
      });
    } catch (e) {
      setStatus(`Cannot open: ${e}`);
    }
  }

  /** Update the breadcrumb trail in the toolbar. */
  function updateBreadcrumbs() {
    const bc = $('#toolbar-breadcrumbs');
    const rel = IDE.currentPath.replace(IDE.sandboxPath, '').replace(/^[/\\]/, '');
    bc.textContent = rel ? `sandbox / ${rel.replace(/[/\\]/g, ' / ')}` : 'sandbox';
  }

  /* ── New file / folder ──────────────────────────────────────────────── */

  async function newFile() {
    const name = await showPrompt('New file name:', 'untitled.py');
    if (!name) return;
    const sep = IDE.currentPath.includes('/') ? '/' : '\\';
    const path = IDE.currentPath + sep + name;
    try {
      await invoke('write_file', { filePath: path, content: '' });
      await load(IDE.currentPath);
      Editor.openFile(path, name, '');
      setStatus(`Created: ${name}`);
    } catch (e) {
      setStatus(`Create failed: ${e}`);
    }
  }

  async function newFolder() {
    const name = await showPrompt('New folder name:', 'new-folder');
    if (!name) return;
    const sep = IDE.currentPath.includes('/') ? '/' : '\\';
    const path = IDE.currentPath + sep + name;
    try {
      await invoke('create_dir', { dirPath: path });
      await load(IDE.currentPath);
      setStatus(`Created folder: ${name}`);
    } catch (e) {
      setStatus(`Create failed: ${e}`);
    }
  }

  /* ── Context Menu ─────────────────────────────────────────────────── */

  function showContextMenu(x, y, entry) {
    // Remove existing context menu if any
    removeContextMenu();

    const menu = document.createElement('div');
    menu.id = 'file-context-menu';
    menu.className = 'context-menu';
    menu.style.left = x + 'px';
    menu.style.top = y + 'px';

    const actions = [
      { label: '✏️ Rename', action: () => renameEntry(entry) },
      { label: '🗑️ Delete', action: () => deleteEntry(entry) },
      { label: '📋 Copy Path', action: () => copyPath(entry) },
    ];

    actions.forEach(({ label, action }) => {
      const item = document.createElement('div');
      item.className = 'context-menu-item';
      item.textContent = label;
      item.addEventListener('click', (e) => {
        e.stopPropagation();
        removeContextMenu();
        action();
      });
      menu.appendChild(item);
    });

    document.body.appendChild(menu);

    // Close on click-away
    setTimeout(() => {
      document.addEventListener('click', removeContextMenu, { once: true });
    }, 0);
  }

  function removeContextMenu() {
    const old = document.getElementById('file-context-menu');
    if (old) old.remove();
  }

  async function renameEntry(entry) {
    const oldName = entry.name;
    const newName = await showPrompt('Rename to:', oldName);
    if (!newName || newName === oldName) return;
    const sep = entry.path.includes('/') ? '/' : '\\';
    const parentDir = entry.path.substring(0, entry.path.lastIndexOf(sep));
    const newPath = parentDir + sep + newName;
    try {
      await invoke('rename_file', { oldPath: entry.path, newPath });
      setStatus(`Renamed: ${oldName} → ${newName}`);
      await load(IDE.currentPath);
    } catch (e) {
      setStatus(`Rename failed: ${e}`);
    }
  }

  async function deleteEntry(entry) {
    const confirmed = confirm(`Delete "${entry.name}"? This cannot be undone.`);
    if (!confirmed) return;
    try {
      await invoke('delete_file', { filePath: entry.path });
      setStatus(`Deleted: ${entry.name}`);
      // Close tab if open
      if (typeof Editor !== 'undefined' && Editor.closeTab) {
        Editor.closeTab(entry.path);
      }
      await load(IDE.currentPath);
    } catch (e) {
      setStatus(`Delete failed: ${e}`);
    }
  }

  function copyPath(entry) {
    if (navigator.clipboard) {
      navigator.clipboard.writeText(entry.path).then(() => {
        setStatus('Path copied to clipboard');
      });
    } else {
      setStatus(entry.path);
    }
  }

  /* ── Wire buttons ───────────────────────────────────────────────────── */

  document.addEventListener('DOMContentLoaded', () => {
    $('#btn-new-file').addEventListener('click', newFile);
    $('#btn-new-folder').addEventListener('click', newFolder);
    $('#btn-refresh').addEventListener('click', () => load(IDE.currentPath));
  });

  return { load };
})();
