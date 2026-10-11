"use strict";

const WHATSAPP_HOME = "https://web.whatsapp.com/";

function activeTab() {
  return browser.tabs.query({ active: true, currentWindow: true }).then((tabs) => tabs[0]);
}

document.getElementById("reload").addEventListener("click", () => {
  activeTab().then((tab) => {
    if (tab) {
      browser.tabs.reload(tab.id).catch(() => {});
    }
    window.close();
  });
});

document.getElementById("home").addEventListener("click", () => {
  activeTab().then((tab) => {
    if (tab) {
      browser.tabs.update(tab.id, { url: WHATSAPP_HOME }).catch(() => {});
    }
    window.close();
  });
});

document.getElementById("fullscreen").addEventListener("click", () => {
  browser.windows.getCurrent().then((win) => {
    const next = win.state === "fullscreen" ? "normal" : "fullscreen";
    browser.windows.update(win.id, { state: next }).catch(() => {});
    window.close();
  });
});
