'use strict';

const api = window.ruwaPanel;
const $ = (id) => document.getElementById(id);
const status = (t) => { $('status').textContent = t; };

function esc(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
  }[c]));
}

async function init() {
  $('close').onclick = () => api.close();

  for (const tab of document.querySelectorAll('#tabs button')) {
    tab.onclick = () => {
      for (const t of document.querySelectorAll('#tabs button')) t.classList.remove('active');
      tab.classList.add('active');
      for (const page of document.querySelectorAll('main section')) {
        page.hidden = page.dataset.page !== tab.dataset.tab;
      }
    };
  }

  const cfg = await api.getConfig();
  $('closeToTray').checked = !!cfg.close_to_tray;
  $('startMinimized').checked = !!cfg.start_minimized;
  $('display').value = cfg.display || 'auto';

  const save = async (patch) => {
    try {
      await api.setConfig(patch);
      status('Saved. Takes effect on next launch.');
    } catch (e) {
      status(`Save failed: ${e.message || e}`);
    }
  };
  $('closeToTray').onchange = (e) => save({ close_to_tray: e.target.checked });
  $('startMinimized').onchange = (e) => save({ start_minimized: e.target.checked });
  $('display').onchange = (e) => save({ display: e.target.value });

  $('css').value = await api.getCss();
  $('saveCss').onclick = async () => {
    try {
      await api.setCss($('css').value);
      status('Custom CSS saved. Reload the page to apply.');
    } catch (e) {
      status(`CSS not saved: ${e.message || e}`);
    }
  };

  const { available, enabled } = await api.listMods();
  const box = $('mods');
  box.innerHTML = '';
  if (!available.length) {
    box.innerHTML = '<p class="hint">No premade mods found in this build.</p>';
  }
  const on = new Set(enabled);
  for (const m of available) {
    const label = document.createElement('label');
    label.className = 'row mod';
    label.innerHTML =
      `<span><strong>${esc(m.name)}</strong> <small>v${esc(m.version)}</small>` +
      (m.description ? `<p>${esc(m.description)}</p>` : '') + '</span>';
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = on.has(m.id);
    cb.onchange = async () => {
      if (cb.checked) on.add(m.id);
      else on.delete(m.id);
      try {
        await api.setMods([...on]);
        status(`Mod '${m.id}' ${cb.checked ? 'enabled' : 'disabled'}. Takes effect on next launch.`);
      } catch (e) {
        status(`Mod toggle failed: ${e.message || e}`);
      }
    };
    label.appendChild(cb);
    box.appendChild(label);
  }

  $('autostart').checked = await api.getAutostart();
  $('autostart').onchange = async (e) => {
    try {
      await api.setAutostart(e.target.checked);
      status('Autostart updated.');
    } catch (err) {
      status(`Autostart failed: ${err.message || err}`);
    }
  };

  $('logout').onclick = async () => {
    try {
      await api.logout();
      status('Logged out. Link your device again.');
    } catch (e) {
      status(`Logout failed: ${e.message || e}`);
    }
  };

  $('version').textContent = `Ruwa ${await api.getVersion()} — WhatsApp desktop client.`;
  $('closePanel').onclick = () => api.close();
}

init().catch((e) => status(`Settings failed to load: ${e.message || e}`));
