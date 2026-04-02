/**
 * ExeOutput JavaScript Bridge
 * Injected into the compiled application.
 */
window.exeoutput = {
  getAppVersion: async () => {
    return await window.__TAURI__.app.getVersion();
  },
  saveFile: async (path, content) => {
    return await window.__TAURI__.fs.writeBinaryFile(path, content);
  },
  openDialog: async (options) => {
    return await window.__TAURI__.dialog.open(options);
  },
  exit: () => {
    window.__TAURI__.process.exit();
  },
  getAppPath: async () => {
    return await window.__TAURI__.path.appDir();
  },
  print: () => {
    window.print();
  },
  showNotification: async (title, body) => {
    return await window.__TAURI__.notification.sendNotification({ title, body });
  },
  sendToBackend: async (command, payload) => {
    return await window.__TAURI__.invoke(command, payload);
  }
};
