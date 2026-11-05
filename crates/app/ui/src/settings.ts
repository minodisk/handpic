import { type AiState, aiStatus } from "./ai.js";
import { type RankModel, confidenceText } from "./meta.js";
import {
  BUG_SVG,
  DATABASE_SVG,
  FILE_TEXT_SVG,
  HARD_DRIVE_DOWNLOAD_SVG,
  KEYBOARD_SVG,
  PLUG_SVG,
  SLIDERS_HORIZONTAL_SVG,
} from "./icons.js";
import { IdleGate } from "./idle.js";
import {
  IMPORT_PATTERN_EXAMPLES,
  IMPORT_SPECIFIERS,
  type ImportSettings,
  rootNote,
} from "./importsettings.js";
import { type Binding, displayKey, keyName } from "./keys.js";
import { LABEL_COLORS, type LabelNames, type LabelPreset, labelNamesPayload } from "./labels.js";
import { type McpState, mcpEndpoint, mcpExamples, mcpStatus } from "./mcp.js";
import { SettingsModal, cycleFocus } from "./modal.js";
import { nextTab } from "./tabs.js";

const shortcutLabels: Record<string, string> = {
  previous: "Previous",
  next: "Next",
  burstPrevious: "Previous burst",
  burstNext: "Next burst",
  burstFramePrevious: "Previous frame in burst",
  burstFrameNext: "Next frame in burst",
  extendPrevious: "Extend selection left",
  extendNext: "Extend selection right",
  selectAll: "Select all",
  open: "Open folder",
  undo: "Undo",
  redo: "Redo",
  toggleLeft: "Show / hide the folder tree",
  toggleRight: "Show / hide the metadata pane",
  toggleStrip: "Show / hide the filmstrip",
  toggleSides: "Show / hide both side panes",
  focus: "Focus mark",
  zoom: "1:1 zoom",
  grayscale: "Grayscale",
  compare: "Compare frames",
  performance: "Performance graph",
  rate1: "1 star",
  rate2: "2 stars",
  rate3: "3 stars",
  rate4: "4 stars",
  rate5: "5 stars",
  reject: "Reject",
  rejectRest: "Reject the rest of the burst",
  pick: "Pick",
  unflag: "Un-reject / un-pick",
  clear: "0 star",
  red: "Red label",
  orange: "Orange label",
  yellow: "Yellow label",
  green: "Green label",
  blue: "Blue label",
  pink: "Pink label",
  purple: "Purple label",
  clearlabel: "Clear label",
  clearall: "Clear",
};

// The settings modal in the main window. `main.ts` hands it every keydown
// while it is open.
export type Settings = {
  readonly isOpen: boolean;
  open(): void;
  close(): void;
  keydown(event: KeyboardEvent): void;
  // `main.ts` reports every change of its `scanRunning` here; the Cache tab
  // holds a clear pressed while a scan runs until it ends.
  setScanRunning(running: boolean): void;
};

// What the modal changes in the main view, applied directly.
export type SettingsHooks = {
  applyKeymap(bindings: Binding[]): void;
  setAutoAdvance(enabled: boolean): void;
  setDebugLogging(enabled: boolean): void;
};

// Looks up the modal's elements and loads its values; call it once.
export function initSettings(hooks: SettingsHooks): Settings {
  const dialog = document.getElementById("settings-dialog") as HTMLDivElement;
  const box = document.getElementById("settings-box") as HTMLDivElement;
  const modal = new SettingsModal();
  let returnFocus: HTMLElement | null = null;
  const shortcutsRows = document.getElementById("shortcuts-rows") as HTMLTableElement;
  const status = document.getElementById("settings-status") as HTMLDivElement;
  const sidecarRadios = document.querySelectorAll<HTMLInputElement>('input[name="sidecar-format"]');
  const labelNamesBlock = document.getElementById("label-names") as HTMLDivElement;
  const labelNameInput = (color: string): HTMLInputElement =>
    document.getElementById(`label-name-${color}`) as HTMLInputElement;
  const labelNamesLanguage = document.getElementById("label-names-language") as HTMLSelectElement;
  let labelPresets: LabelPreset[] = [];
  const autoAdvance = document.getElementById("auto-advance") as HTMLInputElement;
  const aiEnabled = document.getElementById("ai-enabled") as HTMLInputElement;
  const aiStatusLine = document.getElementById("ai-status") as HTMLParagraphElement;
  const rankingSettings = document.getElementById("ranking-settings") as HTMLDivElement;
  const rankingStatus = document.getElementById("ranking-status") as HTMLParagraphElement;
  const rankingRefit = document.getElementById("ranking-refit") as HTMLButtonElement;
  const mcpEnabled = document.getElementById("mcp-enabled") as HTMLInputElement;
  const mcpStatusLine = document.getElementById("mcp-status") as HTMLParagraphElement;
  const mcpEndpointText = document.getElementById("mcp-endpoint") as HTMLPreElement;
  const mcpExamplesBlock = document.getElementById("mcp-examples") as HTMLDivElement;
  const debugTiming = document.getElementById("debug-timing") as HTMLInputElement;
  const importRoot = document.getElementById("import-root") as HTMLInputElement;
  const importRootNote = document.getElementById("import-root-note") as HTMLParagraphElement;
  const importPattern = document.getElementById("import-pattern") as HTMLInputElement;
  const importExample = document.getElementById("import-example") as HTMLParagraphElement;
  const indexSize = document.getElementById("index-size") as HTMLParagraphElement;
  const clearIndex = document.getElementById("clear-index") as HTMLButtonElement;
  const clearIndexNote = document.getElementById("clear-index-note") as HTMLParagraphElement;
  const tablist = document.getElementById("settings-tabs") as HTMLDivElement;
  const tabs = [...tablist.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  const tabIcons: Record<string, string> = {
    "tab-sidecar": FILE_TEXT_SVG,
    "tab-culling": SLIDERS_HORIZONTAL_SVG,
    "tab-shortcuts": KEYBOARD_SVG,
    "tab-cache": DATABASE_SVG,
    "tab-import": HARD_DRIVE_DOWNLOAD_SVG,
    "tab-mcp": PLUG_SVG,
    "tab-debug": BUG_SVG,
  };
  for (const tab of tabs) {
    const icon = document.createElement("span");
    icon.className = "icon";
    icon.innerHTML = tabIcons[tab.id] ?? "";
    tab.prepend(icon);
  }
  let shortcutBindings: Binding[] = [];
  let scanRunning = false;
  let clearInFlight = false;
  // A held clear lasts only while the modal is open: `close()` discards it,
  // so the scan running when the user asked for it never drains it later
  // behind the user's back.
  const clearGate = new IdleGate(() => scanRunning);

  function renderShortcuts(): void {
    shortcutsRows.replaceChildren(
      ...shortcutBindings.map(({ action, keys }) => {
        const row = document.createElement("tr");
        const label = document.createElement("td");
        label.textContent = shortcutLabels[action] ?? action;
        const keysCell = document.createElement("td");
        const keyList = document.createElement("div");
        keyList.className = "keys";
        keysCell.append(keyList);
        const resetCell = document.createElement("td");
        row.append(label, keysCell, resetCell);
        for (const key of keys) {
          const chip = document.createElement("span");
          chip.className = "chip";
          chip.textContent = displayKey(key);
          const remove = document.createElement("button");
          remove.type = "button";
          remove.className = "button ghost";
          remove.textContent = "×";
          remove.setAttribute("aria-label", `Remove ${displayKey(key)}`);
          remove.addEventListener("click", () => {
            void updateShortcuts("remove_shortcut_key", { action, key });
          });
          chip.append(remove);
          keyList.append(chip);
        }
        if (modal.capturing === action) {
          const prompt = document.createElement("span");
          prompt.className = "capturing";
          prompt.textContent = "Press a key...";
          keyList.append(prompt);
        } else {
          const add = document.createElement("button");
          add.type = "button";
          add.className = "button ghost add";
          add.textContent = "+";
          add.setAttribute("aria-label", "Add a key");
          add.addEventListener("click", () => {
            modal.capturing = action;
            status.textContent = "";
            renderShortcuts();
          });
          keyList.append(add);
        }
        const reset = document.createElement("button");
        reset.type = "button";
        reset.className = "button outline";
        reset.textContent = "Reset";
        reset.addEventListener("click", () => {
          void updateShortcuts("reset_shortcut", { action });
        });
        resetCell.append(reset);
        return row;
      }),
    );
  }

  async function updateShortcuts(command: string, args?: Record<string, unknown>): Promise<void> {
    modal.capturing = null;
    try {
      shortcutBindings = await window.__TAURI__.core.invoke<Binding[]>(command, args);
      hooks.applyKeymap(shortcutBindings);
      status.textContent = "";
    } catch (error) {
      status.textContent = String(error);
    }
    renderShortcuts();
  }

  function writesXmp(format: string): boolean {
    return format === "xmp" || format === "both";
  }

  function showSidecarFormat(format: string): void {
    for (const radio of sidecarRadios) {
      radio.checked = radio.value === format;
    }
    labelNamesBlock.hidden = !writesXmp(format);
  }

  function showLabelNames(names: LabelNames): void {
    for (const color of LABEL_COLORS) {
      labelNameInput(color).value = names[color];
    }
  }

  function saveLabelNames(names: LabelNames): void {
    status.textContent = "";
    window.__TAURI__.core
      .invoke<LabelNames>("set_label_names", { names })
      .then(showLabelNames)
      .catch((error: unknown) => {
        status.textContent = String(error);
      });
  }

  void window.__TAURI__.core.invoke<Binding[]>("shortcuts").then((bindings) => {
    shortcutBindings = bindings;
    renderShortcuts();
  });
  void window.__TAURI__.core.invoke<string>("sidecar_format").then(showSidecarFormat);
  void window.__TAURI__.event.listen<string>("sidecar-format", ({ payload }) => {
    showSidecarFormat(payload);
  });
  void window.__TAURI__.core
    .invoke<{ names: LabelNames; presets: LabelPreset[] }>("label_names")
    .then(({ names, presets }) => {
      labelPresets = presets;
      labelNamesLanguage.replaceChildren(
        ...presets.map((preset) => new Option(preset.name, preset.code)),
      );
      showLabelNames(names);
    });
  void window.__TAURI__.core.invoke<boolean>("auto_advance").then((enabled) => {
    autoAdvance.checked = enabled;
  });
  function copyButton(text: HTMLElement): HTMLButtonElement {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "button outline";
    button.textContent = "Copy";
    button.addEventListener("click", () => {
      copyText(text);
    });
    return button;
  }

  // When the webview refuses clipboard access, the text is selected instead so
  // the platform's copy shortcut takes it.
  function copyText(text: HTMLElement): void {
    status.textContent = "";
    navigator.clipboard.writeText(text.textContent ?? "").catch(() => {
      const range = document.createRange();
      range.selectNodeContents(text);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
      status.textContent = "Could not copy; the text is selected, so copy it with the keyboard.";
    });
  }

  function showMcpState(state: McpState): void {
    mcpEnabled.checked = state.enabled;
    mcpStatusLine.textContent = mcpStatus(state);
    mcpEndpointText.textContent = mcpEndpoint(state.port);
    mcpExamplesBlock.replaceChildren(
      ...mcpExamples(state.port).map(({ client, text }) => {
        const block = document.createElement("div");
        block.className = "example";
        const title = document.createElement("p");
        title.textContent = `For example, ${client}:`;
        const copyable = document.createElement("div");
        copyable.className = "copyable";
        const pre = document.createElement("pre");
        pre.textContent = text;
        copyable.append(pre, copyButton(pre));
        block.append(title, copyable);
        return block;
      }),
    );
  }

  function showAiState(state: AiState): void {
    aiEnabled.checked = state.enabled;
    aiStatusLine.textContent = aiStatus(state);
    rankingSettings.hidden = !state.enabled;
  }

  function showRankModel(model: RankModel): void {
    rankingStatus.textContent = `Ranking: ${confidenceText(model)}`;
  }

  void window.__TAURI__.core.invoke<RankModel>("ranking_model").then(showRankModel);
  void window.__TAURI__.event.listen<RankModel>("ranking-updated", ({ payload }) => {
    showRankModel(payload);
  });

  void window.__TAURI__.core.invoke<AiState>("ai_enabled").then(showAiState);
  void window.__TAURI__.event.listen<AiState>("ai-state", ({ payload }) => {
    showAiState(payload);
  });
  void window.__TAURI__.core.invoke<McpState>("mcp_enabled").then(showMcpState);
  void window.__TAURI__.event.listen<McpState>("mcp-state", ({ payload }) => {
    showMcpState(payload);
  });
  void window.__TAURI__.core.invoke<boolean>("debug_build").then((debug) => {
    (document.getElementById("tab-debug") as HTMLButtonElement).hidden = !debug;
  });
  void window.__TAURI__.core.invoke<boolean>("timing_logs").then((enabled) => {
    debugTiming.checked = enabled;
  });

  // Only the newest example request is shown, as the responses of a fast
  // typist's earlier ones can arrive after it.
  let exampleRequest = 0;
  let exampleTimer: ReturnType<typeof setTimeout> | undefined;

  function showImportExample(): void {
    clearTimeout(exampleTimer);
    const request = ++exampleRequest;
    window.__TAURI__.core
      .invoke<string>("import_pattern_example", { pattern: importPattern.value })
      .then(
        (example) => {
          if (request !== exampleRequest) return;
          importExample.classList.remove("error");
          importExample.textContent = example;
        },
        (error: unknown) => {
          if (request !== exampleRequest) return;
          importExample.classList.add("error");
          importExample.textContent = String(error);
        },
      );
  }

  function showImportSettings({ root, pattern }: ImportSettings): void {
    importRoot.value = root;
    const note = rootNote(root);
    importRootNote.textContent = note ?? "";
    importRootNote.hidden = note === null;
    importPattern.value = pattern;
    showImportExample();
  }

  function saveImportPattern(): void {
    status.textContent = "";
    window.__TAURI__.core
      .invoke("set_import_pattern", { pattern: importPattern.value })
      .then(showImportExample, (error: unknown) => {
        exampleRequest++;
        importExample.classList.add("error");
        importExample.textContent = String(error);
      });
  }

  (document.getElementById("import-specifiers") as HTMLTableElement).replaceChildren(
    ...IMPORT_SPECIFIERS.map(({ spec, meaning, example }) => {
      const row = document.createElement("tr");
      const specCell = document.createElement("td");
      const code = document.createElement("code");
      code.textContent = spec;
      specCell.append(code);
      const meaningCell = document.createElement("td");
      meaningCell.textContent = meaning;
      const exampleCell = document.createElement("td");
      exampleCell.textContent = example;
      row.append(specCell, meaningCell, exampleCell);
      return row;
    }),
  );

  (document.getElementById("import-pattern-examples") as HTMLDivElement).replaceChildren(
    ...IMPORT_PATTERN_EXAMPLES.map((pattern) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "button outline";
      button.textContent = pattern;
      button.addEventListener("click", () => {
        importPattern.value = pattern;
        saveImportPattern();
      });
      return button;
    }),
  );

  importPattern.addEventListener("input", () => {
    clearTimeout(exampleTimer);
    exampleTimer = setTimeout(showImportExample, 200);
  });
  importPattern.addEventListener("change", saveImportPattern);

  (document.getElementById("import-root-choose") as HTMLButtonElement).addEventListener(
    "click",
    () => {
      status.textContent = "";
      window.__TAURI__.core
        .invoke<string | null>("pick_folder")
        .then(async (path) => {
          if (path === null) return;
          const settings = await window.__TAURI__.core.invoke<ImportSettings>("set_import_root", {
            path,
          });
          // Keep a pattern still being typed rather than the stored one.
          showImportSettings({ ...settings, pattern: importPattern.value });
        })
        .catch((error: unknown) => {
          status.textContent = String(error);
        });
    },
  );

  void window.__TAURI__.core.invoke<ImportSettings>("import_settings").then(showImportSettings);

  // The backend formats the size in the units of the platform's file manager.
  function showIndexSize(size: string): void {
    indexSize.textContent = `Index cache: ${size}`;
  }

  void window.__TAURI__.core.invoke<string>("index_size").then(showIndexSize);

  function updateClearButton(): void {
    clearIndex.disabled = clearInFlight;
    clearIndexNote.hidden = clearGate.waiting === null;
  }

  function setScanRunning(running: boolean): void {
    const scanEnded = scanRunning && !running;
    scanRunning = running;
    if (!running) {
      clearGate.drain();
    }
    if (scanEnded && !clearInFlight) {
      void window.__TAURI__.core.invoke<string>("index_size").then((size) => {
        if (!clearInFlight) showIndexSize(size);
      });
    }
    updateClearButton();
  }

  for (const radio of sidecarRadios) {
    radio.addEventListener("change", () => {
      status.textContent = "";
      labelNamesBlock.hidden = !writesXmp(radio.value);
      window.__TAURI__.core
        .invoke("set_sidecar_format", { format: radio.value })
        .catch((error: unknown) => {
          status.textContent = String(error);
        });
    });
  }

  for (const color of LABEL_COLORS) {
    labelNameInput(color).addEventListener("change", () => {
      saveLabelNames(labelNamesPayload((c) => labelNameInput(c).value));
    });
  }

  (document.getElementById("label-names-reset") as HTMLButtonElement).addEventListener(
    "click",
    () => {
      const preset = labelPresets.find((p) => p.code === labelNamesLanguage.value);
      if (preset !== undefined) saveLabelNames(preset.names);
    },
  );

  autoAdvance.addEventListener("change", () => {
    status.textContent = "";
    const enabled = autoAdvance.checked;
    window.__TAURI__.core
      .invoke("set_auto_advance", { enabled })
      .then(() => {
        hooks.setAutoAdvance(enabled);
      })
      .catch((error: unknown) => {
        status.textContent = String(error);
      });
  });

  aiEnabled.addEventListener("change", () => {
    status.textContent = "";
    window.__TAURI__.core
      .invoke("set_ai_enabled", { enabled: aiEnabled.checked })
      .catch((error: unknown) => {
        status.textContent = String(error);
      });
  });

  rankingRefit.addEventListener("click", () => {
    rankingRefit.disabled = true;
    rankingStatus.textContent = "Ranking: refitting…";
    window.__TAURI__.core
      .invoke<RankModel>("refit_ranking")
      .then(showRankModel, (error: unknown) => {
        rankingStatus.textContent = String(error);
      })
      .finally(() => {
        rankingRefit.disabled = false;
      });
  });

  mcpEnabled.addEventListener("change", () => {
    status.textContent = "";
    window.__TAURI__.core
      .invoke("set_mcp_enabled", { enabled: mcpEnabled.checked })
      .catch((error: unknown) => {
        status.textContent = String(error);
      });
  });

  (document.getElementById("mcp-endpoint-copy") as HTMLButtonElement).addEventListener(
    "click",
    () => {
      copyText(mcpEndpointText);
    },
  );

  debugTiming.addEventListener("change", () => {
    hooks.setDebugLogging(debugTiming.checked);
    void window.__TAURI__.core.invoke("set_timing_logs", { enabled: debugTiming.checked });
  });

  function clearCache(): void {
    clearInFlight = true;
    updateClearButton();
    window.__TAURI__.core
      .invoke<boolean>("clear_index")
      .then(async (cleared) => {
        if (cleared) {
          showIndexSize(await window.__TAURI__.core.invoke<string>("index_size"));
        }
      })
      .catch(async (error: unknown) => {
        status.textContent = String(error);
        showIndexSize(await window.__TAURI__.core.invoke<string>("index_size"));
      })
      .finally(() => {
        clearInFlight = false;
        updateClearButton();
      });
  }

  clearIndex.addEventListener("click", () => {
    status.textContent = "";
    clearGate.request("Clear Cache", clearCache);
    updateClearButton();
  });

  (document.getElementById("shortcuts-reset-all") as HTMLButtonElement).addEventListener(
    "click",
    () => {
      void updateShortcuts("reset_shortcuts");
    },
  );

  function selectTab(panel: string): void {
    if (modal.capturing !== null) {
      modal.capturing = null;
      renderShortcuts();
    }
    for (const tab of tabs) {
      const selected = tab.getAttribute("aria-controls") === panel;
      tab.setAttribute("aria-selected", String(selected));
      tab.tabIndex = selected ? 0 : -1;
      (document.getElementById(tab.getAttribute("aria-controls") ?? "") as HTMLElement).hidden =
        !selected;
      if (selected) {
        tab.focus();
      }
    }
  }

  for (const tab of tabs) {
    tab.addEventListener("click", () => {
      selectTab(tab.getAttribute("aria-controls") ?? "");
    });
  }

  function focusables(): HTMLElement[] {
    return [...box.querySelectorAll<HTMLElement>("button, input, select, [tabindex]")].filter(
      (el) =>
        el.tabIndex >= 0 &&
        !(el as HTMLButtonElement | HTMLInputElement | HTMLSelectElement).disabled &&
        !(el instanceof HTMLInputElement && el.type === "radio" && !el.checked) &&
        el.getClientRects().length > 0,
    );
  }

  function open(): void {
    if (!modal.open()) {
      return;
    }
    returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    status.textContent = "";
    void window.__TAURI__.core.invoke<string>("sidecar_format").then(showSidecarFormat);
    // The first-launch dialog may have saved names since the modal was built.
    void window.__TAURI__.core
      .invoke<{ names: LabelNames }>("label_names")
      .then(({ names }) => showLabelNames(names));
    if (!clearInFlight) {
      void window.__TAURI__.core.invoke<string>("index_size").then((size) => {
        if (!clearInFlight) showIndexSize(size);
      });
    }
    renderShortcuts();
    dialog.hidden = false;
    tabs.find((tab) => tab.getAttribute("aria-selected") === "true")?.focus();
  }

  function close(): void {
    if (!modal.isOpen) {
      return;
    }
    modal.close();
    renderShortcuts();
    dialog.hidden = true;
    returnFocus?.focus();
    returnFocus = null;
    clearGate.discard();
    updateClearButton();
  }

  // Every key stops here while the modal is open, so none reaches the culling
  // keymap; the ones left `native` do what the focused control does with them.
  function keydown(event: KeyboardEvent): void {
    const decision = modal.key(keyName(event));
    switch (decision.kind) {
      case "add":
        event.preventDefault();
        void updateShortcuts("add_shortcut_key", { action: modal.capturing, key: decision.key });
        return;
      case "cancel":
        event.preventDefault();
        modal.capturing = null;
        renderShortcuts();
        return;
      case "close":
        event.preventDefault();
        close();
        return;
      case "focus": {
        event.preventDefault();
        const elements = focusables();
        if (elements.length === 0) {
          return;
        }
        const from = elements.indexOf(document.activeElement as HTMLElement);
        elements[cycleFocus(elements.length, from, decision.step)].focus();
        return;
      }
      case "native":
        if (tablist.contains(event.target as Node)) {
          tabKey(event);
          return;
        }
        // macOS WKWebView only runs a text input's native Select All through
        // the Edit menu's key equivalent. The menu binds it to the `selectAll`
        // action's key, so rebinding that action away from Cmd+A leaves
        // Cmd+A doing nothing in a focused input here. Cover it directly.
        if (
          keyName(event) === "meta+a" &&
          (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement)
        ) {
          event.preventDefault();
          event.target.select();
        }
    }
  }

  function tabKey(event: KeyboardEvent): void {
    const visible = tabs
      .filter((tab) => !tab.hidden)
      .map((tab) => tab.getAttribute("aria-controls") ?? "");
    const current =
      tabs
        .find((tab) => tab.getAttribute("aria-selected") === "true")
        ?.getAttribute("aria-controls") ?? "";
    const target = nextTab(visible, current, event.key, "vertical");
    if (target !== current) {
      event.preventDefault();
      selectTab(target);
    }
  }

  (document.getElementById("settings-close") as HTMLButtonElement).addEventListener("click", close);

  // A click on the backdrop, outside the box, closes as well.
  dialog.addEventListener("click", (event) => {
    if (event.target === dialog) {
      close();
    }
  });

  return {
    get isOpen() {
      return modal.isOpen;
    },
    open,
    close,
    keydown,
    setScanRunning,
  };
}
