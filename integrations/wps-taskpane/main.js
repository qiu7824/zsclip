(function () {
  const TASKPANE_URL = "http://127.0.0.1:38473/office/wps/taskpane";
  const TASKPANE_STORAGE_KEY = "zsclip_taskpane_id";
  function wpsApplication() {
    if (typeof window !== "undefined" && window.Application) {
      return window.Application;
    }
    if (typeof wps !== "undefined" && typeof wps.WpsApplication === "function") {
      return wps.WpsApplication();
    }
    if (typeof Application !== "undefined") {
      return Application;
    }
    return null;
  }

  function setTaskPaneLayout(app, pane) {
    if (!pane) {
      return;
    }
    try {
      const position = app.Enum && app.Enum.JSKsoEnum_msoCTPDockPositionRight;
      if (position !== undefined) {
        pane.DockPosition = position;
      }
    } catch (_) {}
    try {
      pane.Width = 380;
    } catch (_) {}
    try {
      pane.MinWidth = 360;
    } catch (_) {}
  }

  function openTaskPane() {
    const app = wpsApplication();
    if (!app) {
      throw new Error("WPS Application is unavailable");
    }
    const existingId =
      app.PluginStorage && app.PluginStorage.getItem
        ? app.PluginStorage.getItem(TASKPANE_STORAGE_KEY)
        : "";
    if (existingId && typeof app.GetTaskPane === "function") {
      const existingPane = app.GetTaskPane(existingId);
      if (existingPane) {
        setTaskPaneLayout(app, existingPane);
        existingPane.Visible = true;
        return existingPane;
      }
    }

    const create =
      typeof app.CreateTaskPane === "function"
        ? app.CreateTaskPane.bind(app)
        : typeof app.CreateTaskpane === "function"
          ? app.CreateTaskpane.bind(app)
          : null;
    if (!create) {
      throw new Error("WPS Application.CreateTaskPane is unavailable");
    }
    const pane = create(TASKPANE_URL);
    if (pane) {
      if (app.PluginStorage && app.PluginStorage.setItem && pane.ID) {
        app.PluginStorage.setItem(TASKPANE_STORAGE_KEY, pane.ID);
      }
      setTaskPaneLayout(app, pane);
      try {
        pane.Visible = true;
      } catch (_) {}
    }
    return pane;
  }

  window.OnAddinLoad = function (ribbonUI) {
    const app = wpsApplication();
    if (app && typeof app.ribbonUI !== "object") {
      app.ribbonUI = ribbonUI;
    }
    return true;
  };

  window.OnAction = function (control) {
    if (!control || control.Id === "btnOpenZsclipTaskPane") {
      return openTaskPane();
    }
    return true;
  };

  window.OnOpenZsclipTaskPane = function () {
    return openTaskPane();
  };

  window.ZSClipWpsTaskPane = {
    TASKPANE_URL,
    openTaskPane,
  };
})();
