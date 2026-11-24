<script lang="ts">
  import TemplateChooser from "./TemplateChooser.svelte";
  import TemplateBuilder from "./TemplateBuilder.svelte";
  import UpdateButton from "./UpdateButton.svelte";
  function savedTemplates(){try{return JSON.parse(localStorage.getItem("dss-custom-templates")||"{}");}catch{return {};}}
  let customTemplate=$state<any>({name:"My template",sheets:{}}),templateCatalog=$state<any[]>([]);
  let selectedWorksheets = $derived(templateCatalog.filter(sheet => customTemplate.sheets?.[sheet.name]?.enabled));
  let usesBusinessSheets = $derived(selectedWorksheets.some(sheet => sheet.live));
  let creating = $state(false);
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import { open, save, confirm } from "@tauri-apps/plugin-dialog";
  import Business from "./Business.svelte";
  import { functions, col, address } from "./formulas";
  import rules from "../rules/verified-rates-2026-11-12.json";
  import licenseText from "../LICENSE?raw";
  let licenseDialog: HTMLDialogElement;
  let docs = $state<any[]>([]),
    active = $state(""),
    sheet = $state(0),
    page = $state("home"),
    menu = $state(""),
    modal = $state(""),
    errorText = $state(""),
    status = $state("Ready · all files stay on this computer");
  let theme = $state("system"),
    mode = $state("Simple"),
    zoom = $state(100),
    scale = $state(100),
    layout = $state("Endless");
  let templates = $state<string[]>([]),
    recent = $state<string[]>([]),
    library = $state<{name:string,path:string,modified:number,external?:boolean}[]>([]),
    librarySearch = $state(""),
    recovery = $state<any[]>([]),
    search = $state("");
  let cells = $state<any[]>([]),
    merges = $state<any[]>([]),
    maxRow = $state(1000),
    maxCol = $state(100),
    scrollTop = $state(0),
    scrollLeft = $state(0),
    grid = $state<HTMLDivElement>(null!);
  let row = $state(1),
    column = $state(1),
    endRow = $state(1),
    endCol = $state(1),
    editing = $state(false),
    input = $state(""),
    location = $state("A1"),
    multi = $state<string[]>([]),
    loadSequence = 0;
  let inputDirty = $state(false);
  let functionSearch = $state(""),
    formula = $state(""),
    output = $state("A1"),
    preview = $state(""),
    functionChosen = $state(functions[0]);
  let wizardStep = $state(0),
    chosenTemplate = $state("General Business"),
    business = $state<any>({
      company_id: "",
      name: "",
      country: "South Africa",
      region: "",
      currency: "ZAR",
      kind: "General",
      period: "2026-11",
      year_start: 3,
      accounting: "Accrual",
      registration: "",
      tax_registration: "",
    });
  let genericValue = $state(""),
    genericTitle = $state(""),
    genericAction = $state(""),
    findText = $state("");
  let printRows = $state(50),
    printCols = $state(8),
    paper = $state("A4"),
    orientation = $state("landscape"),
    printRowsData = $state<string[][]>([]),
    printTitle = $state(""),
    printing = $state(false);
  let chartType = $state("Column"),
    chartData = $state<{ label: string; value: number }[]>([]);
  let doc = $derived(docs.find((d) => d.id === active));
  let rowHeight = $derived((30 * zoom) / 100),
    columnWidth = $derived((130 * zoom) / 100);
  let firstRow = $derived(indexAt(scrollTop, false)),
    firstCol = $derived(indexAt(scrollLeft, true));
  let range = $derived({
    sheet,
    row: Math.min(row, endRow),
    column: Math.min(column, endCol),
    height: Math.abs(endRow - row) + 1,
    width: Math.abs(endCol - column) + 1,
  });
  let rangeText = $derived(
    `${col(range.column)}${range.row}${range.width > 1 || range.height > 1 ? ":" + col(range.column + range.width - 1) + (range.row + range.height - 1) : ""}`,
  );
  let visibleRows = $derived(
    Array.from({ length: 40 }, (_, i) => firstRow + i),
  );
  let visibleCols = $derived(
    Array.from({ length: 24 }, (_, i) => firstCol + i),
  );
  let selectedCell = $derived(cells.find((c) => c.r === row && c.c === column));
  let sheetName = $derived(doc?.sheets[sheet]?.name || "Sheet1");
  onMount(() => {
    void initialize();
    const timer = window.setInterval(autosave, 60_000);
    const remove = getCurrentWindow().onCloseRequested(async (e) => {
      if (docs.some((d) => d.dirty) || editing || inputDirty) {
        e.preventDefault();
        if (
          await confirm(
            "Close without saving unsaved changes? Choose Cancel to return and save your work.",
            { title: "Unsaved work", kind: "warning" },
          )
        ) {
          await getCurrentWindow().destroy();
        }
      }
    });
    return () => {
      clearInterval(timer);
      void remove.then((f) => f());
    };
  });
  onMount(() => {
    const remove = listen<string[]>("open-files", (event) => {
      for (const path of event.payload) void openPath(path);
    });
    return () => {
      void remove.then((f) => f());
    };
  });
  $effect(() => {
    if (active && page === "sheet") {
      const id = active,
        s = sheet,
        r = firstRow,
        c = firstCol;
      void loadGrid(id, s, r, c);
    }
  });
  $effect(() => {
    if (theme !== "system") {
      document.documentElement.dataset.theme = theme;
      return;
    }
    const appearance = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme = appearance.matches ? "dark" : "light";
    };
    apply();
    appearance.addEventListener("change", apply);
    return () => appearance.removeEventListener("change", apply);
  });
  $effect(() => {
    document.documentElement.style.fontSize = `${(14 * scale) / 100}px`;
  });
  $effect(() => {
    let style = document.getElementById("print-page-style");
    if (!style) {
      style = document.createElement("style");
      style.id = "print-page-style";
      document.head.append(style);
    }
    style.textContent = `@page {size: ${paper} ${orientation}; margin: 12mm;}`;
  });
  function error(e: unknown) {
    errorText = String(e);
    status = "Action needs attention";
  }
  async function initialize() {
    try {
      const v: any = await invoke("startup");
      templates = [...v.templates,...Object.keys(savedTemplates()).map(n=>"Saved: "+n)];
      recent = v.recent;
      recovery = v.recovery;
      theme = v.settings.theme || "system";
      mode = v.settings.mode || "Simple";
      zoom = v.settings.zoom || 100;
      scale = v.settings.scale || 100;
      for (const p of v.launchFiles) await openPath(p);
    } catch (e) {
      error(e);
    } finally {
      // Commit preferences (or the startup error), then allow a frame to paint
      // at the restored bounds before revealing the native window.
      await tick();
      await new Promise<void>((resolve) => {
        // An unmapped GTK window may receive no paint frames until it is shown.
        const fallback = setTimeout(resolve, 200);
        requestAnimationFrame(() => requestAnimationFrame(() => {
          clearTimeout(fallback);
          resolve();
        }));
      });
      await invoke("show_ready_window").catch(error);
    }
  }
  async function prefs() {
    await invoke("preferences", { value: { theme, mode, zoom, scale } }).catch(
      error,
    );
  }
  function changed(d: any) {
    const i = docs.findIndex((v) => v.id === d.id);
    if (i < 0) docs.push(d);
    else docs[i] = d;
    active = d.id;
  }
  async function loadGrid(id = active, s = sheet, r = firstRow, c = firstCol) {
    const sequence = ++loadSequence;
    try {
      const v: any = await invoke("viewport", {
        id,
        sheet: s,
        row: r,
        column: c,
        height: 40,
        width: 24,
      });
      if (sequence !== loadSequence) return;
      cells = v.cells;
      merges = v.merges;
      columnSizes = v.columnSizes;
      rowSizes = v.rowSizes;
      maxRow = Math.max(maxRow, v.rows + 100, r + 150);
      maxCol = Math.max(maxCol, v.cols + 20, c + 30);
    } catch (e) {
      error(e);
    }
  }
  async function showAddSheets(){try{templateCatalog=await invoke<any[]>("template_catalog");customTemplate=doc.metadata.settings?.custom_template?structuredClone($state.snapshot(doc.metadata.settings.custom_template)):{name:doc.metadata.template,sheets:Object.fromEntries(doc.sheets.map((s:any)=>[s.name,{enabled:true,columns:{}}]))};if(!doc.metadata.business){business={...business,company_id:crypto.randomUUID(),name:doc.name,period:new Date().toISOString().slice(0,7)};}menu="";modal="addSheets";}catch(e){error(e);}}
  async function attachSheets(){try{changed(await invoke("add_worksheets",{id:doc.id,config:$state.snapshot(customTemplate),business:doc.metadata.business||(usesBusinessSheets?$state.snapshot(business):null)}));modal="";resetSheet();page=doc.metadata.business?"business":"sheet";}catch(e){error(e);}}
  async function newWorkbook(name?: string) {
    if (page === "sheet" && inputDirty && !(await commit())) return;
    rolloverSource = null;
    if (!name) { menu = ""; modal = "chooseWorkbook"; return; }
    const savedName=name.startsWith("Saved: ")?name.slice(7):null;
    if(savedName)name="Custom Template";
    if (
      [
        "Custom Template",
        "Security Company",
        "Retail Company",
        "Service Company",
        "General Business",
      ].includes(name)
    ) {
      chosenTemplate = name;
      if(name==="Custom Template"){templateCatalog=await invoke<any[]>("template_catalog");customTemplate=savedName?structuredClone(savedTemplates()[savedName]):{name:"My template",sheets:{}};}
      business = {
        ...business,
        company_id: crypto.randomUUID(),
        name: "",
        kind: name === "Custom Template" ? "General" : name.replace(" Company", "").replace(" Business", ""),
      };
      wizardStep = 0;
      modal = name === "Custom Template" ? "customSetup" : "wizard";
      return;
    }
    try {
      changed(
        await invoke("create_workbook", { template: name, business: null }),
      );
      modal = "";
      resetSheet();
      page = "sheet";
    } catch (e) {
      error(e);
    }
  }
  function resetSheet() {
    sheet = 0;
    row = column = endRow = endCol = 1;
    scrollTop = scrollLeft = 0;
    input = "";
    inputDirty = false;
    editing = false;
    if (grid) {
      grid.scrollTop = 0;
      grid.scrollLeft = 0;
    }
  }
  let rolloverSource = $state<string | null>(null);
  async function finishWizard() {
    if (creating) return;
    creating = true;
    const custom = chosenTemplate === "Custom Template";
    const needsBusiness = !custom || usesBusinessSheets || rolloverSource !== null;
    try {
      changed(
        await invoke("create_workbook", {
          template: chosenTemplate,
          business: needsBusiness ? $state.snapshot(business) : null,
          sourceId: rolloverSource,
          customTemplate: chosenTemplate==="Custom Template" ? $state.snapshot(customTemplate) : null,
        }),
      );
      if(chosenTemplate==="Custom Template"){const saved=savedTemplates();saved[customTemplate.name||"My template"]=$state.snapshot(customTemplate);localStorage.setItem("dss-custom-templates",JSON.stringify(saved));templates=[...templates.filter(t=>!t.startsWith("Saved: ")),...Object.keys(saved).map(n=>"Saved: "+n)];}
      rolloverSource = null;
      modal = "";
      resetSheet();
      page = needsBusiness ? "business" : "sheet";
    } catch (e) {
      error(e);
    } finally { creating = false; }
  }
  function newMonth() {
    rolloverSource = doc.id;
    business = {
      ...doc.metadata.business,
      period: nextMonth(doc.metadata.business.period),
    };
    chosenTemplate = doc.metadata.template;
    if(chosenTemplate==="Custom Template"){customTemplate=structuredClone($state.snapshot(doc.metadata.settings.custom_template));void invoke<any[]>("template_catalog").then(c=>templateCatalog=c);}
    wizardStep = 0;
    modal = "wizard";
  }
  function nextMonth(p: string) {
    let [y, m] = p.split("-").map(Number);
    m++;
    if (m > 12) {
      m = 1;
      y++;
    }
    return `${y}-${String(m).padStart(2, "0")}`;
  }
  async function chooseOpen() {
    try {
      const paths = await open({
        multiple: true,
        filters: [
          { name: "Spreadsheet workbooks", extensions: ["xlsx", "ods"] },
        ],
      });
      if (paths)
        for (const path of Array.isArray(paths) ? paths : [paths])
          await openPath(path);
    } catch (e) {
      error(e);
    }
  }
  async function openPath(path: string) {
    try {
      if (page === "sheet" && inputDirty && !(await commit())) return;
      changed(await invoke("open_workbook", { path }));
      resetSheet();
      page = "sheet";
      status =
        "Opened workbook · Save stores your changes in My workbooks";
    } catch (e) {
      error(e);
    }
  }
  async function refreshLibrary() {
    library = await invoke("list_workbooks");
  }
  async function showLibrary() {
    if (page === "sheet" && (editing || inputDirty) && !(await commit())) return;
    try {
      await refreshLibrary();
      librarySearch = "";
      menu = "";
      page = "library";
    } catch (e) { error(e); }
  }
  async function storeWorkbook(id: string, path: string | null = null) {
    const before = docs.find(d => d.id === id);
    const saved = await invoke<any>("save_workbook", { id, path });
    const index = docs.findIndex(d => d.id === id);
    // A later edit or tab close can finish while a save is in flight.
    if (index >= 0 && docs[index] === before) {
      docs[index].path = saved.path;
      docs[index].name = saved.name;
      docs[index].dirty = false;
    }
  }
  let pendingAutosave: Promise<void> | null = null;
  function autosave() {
    pendingAutosave ??= autosaveWorkbooks().finally(() => { pendingAutosave = null; });
    return pendingAutosave;
  }
  async function autosaveWorkbooks() {
    try {
      if (page === "sheet" && inputDirty && doc) {
        const draft = { id: doc.id, sheet, row, column, value: input };
        const before = doc;
        try {
          const updated = await invoke<any>("action", {
            id: draft.id, kind: "set", args: { ...draft, width: 1, height: 1 }
          });
          const index = docs.findIndex(d => d.id === draft.id);
          if (index >= 0 && docs[index] === before) docs[index] = updated;
          if (active === draft.id && sheet === draft.sheet && row === draft.row &&
              column === draft.column && input === draft.value) inputDirty = false;
        } catch (e) { error(e); }
      }
      for (const id of docs.filter(d => d.dirty).map(d => d.id)) {
        try {
          await storeWorkbook(id);
          status = "Autosaved at " + new Date().toLocaleTimeString();
        } catch (e) { error(e); }
      }
      if (page === "library") await refreshLibrary();
    } catch (e) { error(e); }
  }
  async function saveBeforeUpdate() {
    if (modal || document.querySelector(".business-modal")) {
      throw new Error("Finish the open dialog before updating.");
    }
    await pendingAutosave;
    if (doc && (editing || inputDirty) && !(await commit())) {
      throw new Error("Your cell edit could not be saved. Review it before updating.");
    }
    for (const workbook of docs.filter(d => d.dirty)) {
      await storeWorkbook(workbook.id);
    }
    if (editing || inputDirty || docs.some(d => d.dirty)) {
      throw new Error("A workbook still has unsaved changes. Save it before updating.");
    }
  }
  async function saveDoc(as = false) {
    if (!doc) return;
    const id = doc.id;
    try {
      if (page === "sheet" && (editing || inputDirty) && !(await commit())) return;
      let path: string | null = null;
      if (as) {
        path = await save({
          defaultPath: doc.name.replace(/\.(xlsx|ods)$/i, "") + ".xlsx",
          filters: [
            { name: "Excel workbook", extensions: ["xlsx"] },
            { name: "OpenDocument spreadsheet", extensions: ["ods"] },
          ],
        });
        if (!path) return;
        if (!(await confirm("Export supported cells, formulas and business metadata? Imported charts, drawings and some formatting may be lost in the exported copy.", {title:"Export workbook"}))) return;
      }
      await storeWorkbook(id, path);
      status = path ? "Copy exported and workbook saved in My workbooks" : "Saved in My workbooks";
      if (page === "library") await refreshLibrary();
    } catch (e) { error(e); }
  }
  async function closeDoc(id: string) {
    const d = docs.find((x) => x.id === id);
    if (
      (d?.dirty || (id === active && inputDirty)) &&
      !(await confirm("Close this workbook and discard its unsaved changes?"))
    )
      return;
    try { await invoke("close_workbook", { id }); }
    catch (e) { error(e); return; }
    docs = docs.filter((x) => x.id !== id);
    active = docs.at(-1)?.id || "";
    resetSheet();
    if (!active) page = "home";
  }
  async function act(kind: string, args: any = {}) {
    const id = active;
    try {
      const updated = await invoke<any>("action", { id, kind, args: { ...range, ...args } });
      const index = docs.findIndex(d => d.id === id);
      if (index >= 0) docs[index] = updated;
      if (active === id && page === "sheet") await loadGrid();
      status = "Updated · recovery snapshot saved";
      return true;
    } catch (e) {
      error(e);
      return false;
    }
  }
  async function selectCell(r: number, c: number, event?: MouseEvent) {
    if (editing) await commit();
    if (event?.shiftKey) {
      endRow = r;
      endCol = c;
    } else {
      row = endRow = r;
      column = endCol = c;
    }
    location = `${col(column)}${row}`;
    if (event?.ctrlKey)
      multi.push(`'${sheetName.replaceAll("'", "''")}'!${location}`);
    try {
      const selection = { id: active, sheet, row, column };
      const previous = input;
      const v: any = await invoke("cell", selection);
      if (active !== selection.id || sheet !== selection.sheet || row !== selection.row ||
          column !== selection.column || input !== previous) return;
      input = v.input;
      inputDirty = false;
    } catch (e) {
      error(e);
    }
  }
  async function commit() {
    const draft = { id: active, sheet, row, column, value: input };
    const ok = await act("set", {
      row,
      column,
      width: 1,
      height: 1,
      value: input,
    });
    if (ok && active === draft.id && sheet === draft.sheet && row === draft.row &&
        column === draft.column && input === draft.value) {
      editing = false;
      inputDirty = false;
    }
    return ok;
  }
  function startEdit(c: any) {
    row = endRow = c.r;
    column = endCol = c.c;
    input = c.input;
    editing = true;
  }
  function selected(r: number, c: number) {
    return (
      r >= range.row &&
      r < range.row + range.height &&
      c >= range.column &&
      c < range.column + range.width
    );
  }
  function scroll(e: Event) {
    const target = e.currentTarget as HTMLDivElement;
    scrollTop = target.scrollTop;
    scrollLeft = target.scrollLeft;
  }
  async function go() {
    try {
      const p = address(location);
      row = endRow = p.r;
      column = endCol = p.c;
      maxRow = Math.max(maxRow, p.r + 150);
      maxCol = Math.max(maxCol, p.c + 30);
      await new Promise((r) => setTimeout(r, 0));
      grid.scrollTop = offset(p.r, false);
      grid.scrollLeft = offset(p.c, true);
      await selectCell(p.r, p.c);
    } catch (e) {
      error(e);
    }
  }
  let internalClipboard: any = null;
  async function copy(cut = false) {
    try {
      const clip: any = await invoke("copy_cells", { id: active, area: range });
      await navigator.clipboard.writeText(clip.csv);
      internalClipboard = { id: active, clip, cut };
      status = cut
        ? "Cut selection · choose destination and paste"
        : "Copied selection with formulas and formatting";
    } catch (e) {
      error(e);
    }
  }
  async function paste() {
    try {
      const text = await navigator.clipboard.readText();
      if (
        internalClipboard?.id === active &&
        text === internalClipboard.clip.csv
      ) {
        await act("pasteInternal", {
          clipboard: internalClipboard.clip,
          cut: internalClipboard.cut,
        });
        if (internalClipboard.cut) internalClipboard = null;
      } else await act("paste", { value: text });
    } catch (e) {
      error(e);
    }
  }
  async function keydown(e: KeyboardEvent) {
    if (document.querySelector("dialog[open]")) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "n") {
      e.preventDefault();
      if (!modal) void newWorkbook();
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      void saveDoc(e.shiftKey);
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key === "o") {
      e.preventDefault();
      void chooseOpen();
      return;
    }
    if (page !== "sheet" || !doc) return;
    if ((e.target as HTMLElement)?.closest("input, textarea, select, .top-actions")) return;
    if (e.ctrlKey || e.metaKey) {
      if (["c", "x", "v", "z", "y"].includes(e.key)) {
        e.preventDefault();
        if (e.key === "c") void copy();
        if (e.key === "x") void copy(true);
        if (e.key === "v") void paste();
        if (e.key === "z") void act("undo");
        if (e.key === "y") void act("redo");
      }
      return;
    }
    const movement: any = {
      ArrowDown: [1, 0],
      ArrowUp: [-1, 0],
      ArrowLeft: [0, -1],
      ArrowRight: [0, 1],
      Enter: [1, 0],
      Tab: [0, e.shiftKey ? -1 : 1],
    };
    if (movement[e.key]) {
      e.preventDefault();
      const [dr, dc] = movement[e.key];
      const nr = Math.max(1, Math.min(1048576, row + dr)),
        nc = Math.max(1, Math.min(16384, column + dc));
      await selectCell(nr, nc);
      if (nr < firstRow || nr > firstRow + 20)
        grid.scrollTop = offset(nr, false);
      if (nc < firstCol || nc > firstCol + 8)
        grid.scrollLeft = offset(nc, true);
    } else if (e.key === "Delete" || e.key === "Backspace") {
      e.preventDefault();
      void act("clear");
    } else if (e.key === "F2") {
      editing = true;
    } else if (e.key.length === 1 && !e.altKey) {
      input = e.key;
      inputDirty = true;
      editing = true;
    }
  }
  function style(path: string, value: string) {
    void act("style", { path, value });
  }
  function showInput(title: string, action: string, value = "") {
    genericTitle = title;
    genericAction = action;
    genericValue = value;
    modal = "input";
    menu = "";
  }
  async function runGeneric() {
    if (genericAction === "find") {
      await act("replace", { find: findText, value: genericValue });
    } else
      await act(genericAction, {
        value:
          genericAction.includes("Width") || genericAction.includes("Height")
            ? Number(genericValue)
            : genericValue,
      });
    modal = "";
  }
  function insertFunction() {
    functionSearch = "";
    functionChosen = functions[0];
    const references = multi.length
      ? multi.join(",")
      : `'${sheetName.replaceAll("'", "''")}'!${rangeText}`;
    formula = `=SUM(${references})`;
    output = location;
    preview = "";
    modal = "functions";
    menu = "";
  }
  function chooseFunction(f: any) {
    functionChosen = f;
    const refs = multi.length
      ? multi.join(",")
      : `'${sheetName.replaceAll("'", "''")}'!${rangeText}`;
    formula = ["SUM", "AVERAGE", "MAX", "MIN", "COUNT", "COUNTA"].includes(
      f.name,
    )
      ? `=${f.name}(${refs})`
      : "=" + f.example;
    preview = "";
  }
  async function formulaPreview() {
    try {
      const p = address(output);
      const v: any = await invoke("preview_formula", {
        id: active,
        sheet,
        row: p.r,
        column: p.c,
        formula,
      });
      preview = v.value;
    } catch (e) {
      error(e);
    }
  }
  async function insertMapped() {
    try {
      const p = address(output);
      await act("set", {
        row: p.r,
        column: p.c,
        width: 1,
        height: 1,
        value: formula,
      });
      modal = "";
      multi = [];
    } catch (e) {
      error(e);
    }
  }
  async function explain() {
    formula = input || selectedCell?.input || "";
    modal = "explain";
  }
  async function showChart() {
    try {
      chartData = [];
      for (
        let r = range.row;
        r < range.row + Math.min(range.height, 100);
        r++
      ) {
        const a: any = await invoke("cell", {
          id: active,
          sheet,
          row: r,
          column: range.column,
        });
        const b: any = await invoke("cell", {
          id: active,
          sheet,
          row: r,
          column: range.column + 1,
        });
        const n = Number(b.text.replace(/,/g, ""));
        if (Number.isFinite(n)) chartData.push({ label: a.text, value: n });
      }
      modal = "chart";
    } catch (e) {
      error(e);
    }
  }
  async function showPrint() {
    printRows = Math.min(maxRow, 50);
    printCols = 8;
    modal = "print";
    menu = "";
  }
  async function printPreview() {
    try {
      printRowsData = await invoke("print_data", {
        id: active,
        sheet,
        rows: printRows,
        columns: printCols,
      });
      printTitle = sheetName;
      printing = true;
      modal = "";
    } catch (e) {
      error(e);
    }
  }
  async function nativePrint() {
    await invoke("native_print").catch(error);
  }
  async function recoverItem(id: string, restore: boolean) {
    try {
      const v = await invoke("recover", { id, restore });
      recovery = recovery.filter((r) => r.id !== id);
      if (restore) {
        changed(v);
        page = "sheet";
      }
    } catch (e) {
      error(e);
    }
  }
  let columnSizes = $state<any[]>([]),
    rowSizes = $state<any[]>([]);
  function sizeAt(n: number, horizontal: boolean) {
    return (
      (Math.max(
        12,
        (horizontal ? columnSizes : rowSizes).find(
          (s) => n >= s.start && n <= s.end,
        )?.size || (horizontal ? 130 : 30),
      ) *
        zoom) /
      100
    );
  }
  function offset(n: number, horizontal: boolean) {
    const base = horizontal ? 130 : 30;
    let amount = (n - 1) * base;
    for (const s of horizontal ? columnSizes : rowSizes) {
      const count = Math.max(0, Math.min(n - 1, s.end) - s.start + 1);
      amount += count * (Math.max(12, s.size) - base);
    }
    return (amount * zoom) / 100;
  }
  function indexAt(position: number, horizontal: boolean) {
    let low = 1,
      high = horizontal ? 16384 : 1048576;
    while (low < high) {
      const middle = Math.ceil((low + high) / 2);
      if (offset(middle, horizontal) <= position) low = middle;
      else high = middle - 1;
    }
    return low;
  }
  function focusEditor(node: HTMLInputElement) {
    node.focus();
    node.setSelectionRange(node.value.length, node.value.length);
  }
  function cellStyle(c: any) {
    const f = c.style?.font || {};
    const bg =
      c.fill && c.fill.toUpperCase() !== "#FFFFFF" ? c.fill : "var(--cell)";
    const color =
      c.color && c.color !== "#000000" && c.color !== "#00000000"
        ? c.color
        : "var(--text)";
    const merge = merges.find(
      (m) =>
        c.r >= m.row &&
        c.r < m.row + m.height &&
        c.c >= m.column &&
        c.c < m.column + m.width,
    );
    if (merge && (merge.row !== c.r || merge.column !== c.c))
      return "display:none";
    const w = merge
      ? offset(c.c + merge.width, true) - offset(c.c, true)
      : sizeAt(c.c, true);
    const h = merge
      ? offset(c.r + merge.height, false) - offset(c.r, false)
      : sizeAt(c.r, false);
    return `left:${offset(c.c, true) + 48}px;top:${offset(c.r, false) + 30}px;width:${w}px;height:${h}px;background:${bg};color:${color};z-index:${merge ? 2 : 1};font-weight:${f.b ? 700 : 400};font-style:${f.i ? "italic" : "normal"};text-decoration:${f.u ? "underline" : f.strike ? "line-through" : "none"};text-align:${c.style?.alignment?.horizontal?.toLowerCase() || "left"};font-size:${((f.sz || 12) * zoom) / 100}px;white-space:${c.style?.alignment?.wrap_text ? "normal" : "nowrap"}`;
  }
</script>

<svelte:window onkeydown={keydown} />
<dialog bind:this={licenseDialog} class="app-license" aria-labelledby="license-title">
  <header>
    <h2 id="license-title">Application license</h2>
    <button onclick={() => licenseDialog.close()}>Close</button>
  </header>
  <textarea readonly aria-label="License text" value={licenseText}></textarea>
</dialog>
<div class="app" class:simple={mode === "Simple"}>
  <aside class="rail">
    <button
      class="brand"
      title="Document Studio"
      onclick={() => (page = "home")}
      ><img src="/icon.png" alt="Document Studio" /></button
    ><button
      class:active={page === "home"}
      title="Home and templates"
      onclick={() => (page = "home")}>⌂<small>Home</small></button
    ><button
      class:active={page === "library"} title="My workbooks" onclick={showLibrary}>▤<small>Workbooks</small></button
    ><button
      class:active={page === "sheet"}
      title="Spreadsheet"
      onclick={() => {
        if (doc) page = "sheet";
        else void newWorkbook();
      }}>▦<small>Sheets</small></button
    >{#if doc?.metadata.business}<button
        class:active={page === "business"}
        title="Business workspace"
        onclick={() => (page = "business")}>▣<small>Business</small></button
      >{/if}
    <div class="rail-bottom">
      <button title="Rules and sources" onclick={() => (modal = "rules")}
        >§<small>Rules</small></button
      ><button title="Preferences" onclick={() => (modal = "settings")}
        >⚙<small>Settings</small></button
      >
    </div>
  </aside>
  <main>
    <header class="topbar">
      <div>
        <strong>Document Studio <span>Spreadsheets</span></strong><small
          >YOUR IDEAS. BEAUTIFULLY CALCULATED.</small
        >
      </div>
      <div class="top-actions">
        {#if doc && page !== "sheet"}<button onclick={() => saveDoc()}>Save workbook</button>{/if}<button
          onclick={() => {
            mode = mode === "Simple" ? "Advanced" : "Simple";
            void prefs();
          }}>{mode} mode ▾</button
        ><button class="primary" onclick={() => void newWorkbook()}
          >+ New workbook</button
        >
        <UpdateButton {saveBeforeUpdate} />
        <button class="support-button" type="button" title="Support Forbidden_Fantasy on Ko-fi (opens in your browser)"
          onclick={() => invoke("open_support_page").catch(error)}>
          <img src="/kofi-logo.png" alt="" />
          Support Me
        </button>
      </div>
    </header>
    {#if docs.length}<nav class="workbook-tabs">
        {#each docs as d}<button
            class:active={active === d.id}
            onclick={() => {
              active = d.id;
              resetSheet();
              page = "sheet";
            }}
            >{d.dirty ? "● " : ""}{d.name}<span
              role="button"
              tabindex="0"
              aria-label="Close workbook"
              onclick={(e) => {
                e.stopPropagation();
                void closeDoc(d.id);
              }}
              onkeydown={(e) => {
                if (e.key === "Enter") void closeDoc(d.id);
              }}>×</span
            ></button
          >{/each}
      </nav>{/if}
    {#if page === "library"}
      <section class="home">
        <div class="section-heading"><div><h1>My workbooks</h1>
          <p>Workbooks are stored on this computer. Changes are saved every minute.</p></div>
          <button onclick={chooseOpen}>Open a file…</button></div>
        <input aria-label="Search saved workbooks" placeholder="Search workbooks…" bind:value={librarySearch} />
        <div class="recent">
          {#each library.filter(entry => entry.name.toLowerCase().includes(librarySearch.trim().toLowerCase())) as entry (entry.path)}
            <button onclick={() => openPath(entry.path)}>▤ <span>{entry.name}<small>{entry.external ? "Recent file · " + entry.path : "Saved " + new Date(entry.modified).toLocaleString()}</small></span><b>Open ↗</b></button>
          {:else}<p>{library.length ? "No matching workbooks." : "Your first workbook will appear here when you create it."}</p>{/each}
        </div>
      </section>
    {:else if page === "home"}
      <section class="home">
        <div class="hero">
          <div>
            <p class="eyebrow">A LITTLE STRUCTURE. ENDLESS POSSIBILITIES.</p>
            <h1>Make room for<br /><em>your next big idea.</em></h1>
            <p>
              From everyday lists to your monthly business numbers.<br />Start
              simply, and build something that works for you.
            </p>
            <div class="hero-actions">
              <button class="primary" onclick={() => void newWorkbook("Blank workbook")}
                >＋ Blank workbook</button
              ><button onclick={chooseOpen}>Open a workbook</button>
            </div>
          </div>
          <img src="/icon.png" alt="Glossy purple spreadsheet document" />
        </div>
        {#if recovery.length}<section class="card recovery">
            <h3>Unsaved work recovered after an unexpected closure</h3>
            {#each recovery as r}<div class="panel-tools">
                <span>{r.path || r.metadata.template}</span><button
                  onclick={() => recoverItem(r.id, true)}>Restore</button
                ><button onclick={() => recoverItem(r.id, false)}
                  >Discard</button
                >
              </div>{/each}
          </section>{/if}
        <div class="section-heading">
          <div>
            <p class="eyebrow">START WITH A LITTLE HELP</p>
            <h2>A workbook for every day</h2>
          </div>
          <input
            placeholder="Search templates…"
            aria-label="Search templates"
            bind:value={search}
          />
        </div>
        <div class="template-grid">
          {#each templates.filter((t) => t !== "Blank workbook" && t
                .toLowerCase()
                .includes(search.toLowerCase())) as name, i}<button
              class="template-card"
              onclick={() => newWorkbook(name)}
              ><div class="template-art">
                <span>{["▦", "▥", "◈", "▤"][i % 4]}</span>
                <div class="mini-grid"></div>
              </div>
              <strong>{name}</strong><small
                >{name.includes("Company") || name === "General Business"
                  ? "Guided business setup"
                  : "Ready-to-use calculations"} <span>↗</span></small
              ></button
            >{/each}
        </div>
        {#if recent.length}<h2>Pick up where you left off</h2>
          <div class="recent">
            {#each recent as path}<button onclick={() => openPath(path)}
                >▤ <span>{path.split(/[\\/]/).at(-1)}<small>{path}</small></span
                ><b>Open ↗</b></button
              >{/each}
          </div>{/if}
      </section>
    {:else if page === "business" && doc?.metadata.business}<Business
        {doc}
        {changed}
        {error}
        {newMonth}
        openSheet={(s: number) => { sheet = s; firstRow = 1; firstCol = 1; page = "sheet"; void loadGrid(active, s, 1, 1); }}
      />
    {:else if doc}
      <nav class="menubar">
        {#each ["File", "Edit", "Insert", "Format", "Sheet", "View", "Help"] as item}<div
            class="menu-parent"
          >
            <button
              class:active={menu === item}
              onclick={() => (menu = menu === item ? "" : item)}>{item}</button
            >{#if menu === item}<div class="dropdown">
                {#if item === "File"}<button
                    onclick={() => {
                      menu = "";
                      void newWorkbook();
                    }}>New workbook <kbd>Ctrl N</kbd></button
                  ><button
                    onclick={() => {
                      menu = "";
                      void chooseOpen();
                    }}>Open… <kbd>Ctrl O</kbd></button
                  ><button
                    onclick={() => {
                      menu = "";
                      void saveDoc();
                    }}>Save <kbd>Ctrl S</kbd></button
                  ><button
                    onclick={() => {
                      menu = "";
                      void saveDoc(true);
                    }}>Save a copy as…</button
                  ><button onclick={showPrint}>Print preview…</button><button onclick={showAddSheets}>Add worksheets from templates…</button>
                {:else if item === "Edit"}<button
                    onclick={() => {
                      menu = "";
                      void act("undo");
                    }}>Undo</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void act("redo");
                    }}>Redo</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void copy();
                    }}>Copy</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void copy(true);
                    }}>Cut</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void paste();
                    }}>Paste</button
                  ><button
                    onclick={() =>
                      showInput("Replace text in selection", "find")}
                    >Find and replace selection…</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void act("clear");
                    }}>Clear contents</button
                  >
                {:else if item === "Insert"}<button onclick={insertFunction}
                    >ƒ Functions & guided mapping</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void showChart();
                    }}>Chart from selected two columns…</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void act("insertRows");
                    }}>Insert selected rows</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void act("insertColumns");
                    }}>Insert selected columns</button
                  >
                {:else if item === "Format"}<button
                    onclick={() => {
                      menu = "";
                      void act("merge");
                    }}>Merge selected cells</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void act("unmerge");
                    }}>Unmerge cells</button
                  ><button
                    onclick={() => {
                      menu = "";
                      style("alignment.wrap_text", "true");
                    }}>Wrap text</button
                  ><button
                    onclick={() => showInput("Name this range", "defineName")}
                    >Create named range…</button
                  ><button
                    onclick={() =>
                      showInput("Custom number format", "format", "#,##0.00")}
                    >Number format…</button
                  ><button
                    onclick={() =>
                      showInput("Column width in pixels", "columnWidth", "130")}
                    >Column width…</button
                  ><button
                    onclick={() =>
                      showInput("Row height in pixels", "rowHeight", "30")}
                    >Row height…</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void act("fillDown");
                    }}>Fill down</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void act("fillRight");
                    }}>Fill right</button
                  >
                {:else if item === "Sheet"}<button
                    onclick={() => {
                      menu = "";
                      void act("addSheet");
                    }}>Add worksheet</button
                  ><button
                    onclick={() =>
                      showInput("Rename worksheet", "renameSheet", sheetName)}
                    >Rename worksheet…</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void act("duplicateSheet");
                    }}>Duplicate worksheet</button
                  ><button
                    onclick={async () => {
                      menu = "";
                      if (await confirm("Delete this worksheet?"))
                        await act("deleteSheet");
                      sheet = 0;
                    }}>Delete worksheet</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void act("deleteRows");
                    }}>Delete selected rows</button
                  ><button
                    onclick={() => {
                      menu = "";
                      void act("deleteColumns");
                    }}>Delete selected columns</button
                  >
                {:else if item === "View"}<button
                    onclick={() => {
                      menu = "";
                      layout = "Endless";
                    }}>Endless layout</button
                  ><button
                    onclick={() => {
                      menu = "";
                      layout = "A4 Paginated";
                    }}>A4 page guides (landscape)</button
                  ><button
                    onclick={() => {
                      menu = "";
                      modal = "settings";
                    }}>Appearance and accessibility…</button
                  >
                {:else}<button
                    onclick={() => {
                      menu = "";
                      modal = "help";
                    }}>Keyboard shortcuts & help</button
                  ><button
                    onclick={() => {
                      menu = "";
                      modal = "limitations";
                    }}>Feature support and limitations</button
                  ><button
                    onclick={() => {
                      menu = "";
                      modal = "rules";
                    }}>Official tax-rule sources</button
                  ><button onclick={() => { menu = ""; licenseDialog.showModal(); }}>Application license</button>{/if}
              </div>{/if}
          </div>{/each}<span class="menu-spacer"></span><button
          onclick={() => saveDoc()}>Save workbook</button
        >
      </nav>
      <div class="toolbar">
        <div class="tool-group">
          <button
            title="Undo"
            disabled={!doc.canUndo}
            onclick={() => act("undo")}>↶</button
          ><button
            title="Redo"
            disabled={!doc.canRedo}
            onclick={() => act("redo")}>↷</button
          >
        </div>
        <div class="tool-group">
          <button
            title="Bold"
            onclick={() =>
              style("font.b", String(!selectedCell?.style?.font?.b))}
            ><b>B</b></button
          ><button
            title="Italic"
            onclick={() =>
              style("font.i", String(!selectedCell?.style?.font?.i))}
            ><i>I</i></button
          ><button
            title="Underline"
            onclick={() =>
              style("font.u", String(!selectedCell?.style?.font?.u))}
            ><u>U</u></button
          ><label class="color-tool" title="Cell background"
            >Fill<input
              type="color"
              value="#4c236b"
              onchange={(e) => style("fill.color", e.currentTarget.value)}
            /></label
          ><label class="color-tool" title="Text color"
            >Text<input
              type="color"
              value="#ffffff"
              onchange={(e) => style("font.color", e.currentTarget.value)}
            /></label
          >
        </div>
        <div class="tool-group">
          <select
            aria-label="Number format"
            onchange={(e) => style("num_fmt", e.currentTarget.value)}
            ><option value="General">General</option><option value="#,##0.00"
              >Number</option
            ><option value="0.00%">Percentage</option><option value="yyyy-mm-dd"
              >Date</option
            ><option value="hh:mm">Time</option><option value="R #,##0.00"
              >ZAR</option
            ><option value="$#,##0.00">USD</option><option value="£#,##0.00"
              >GBP</option
            ><option value="€#,##0.00">EUR</option></select
          >{#if mode === "Advanced"}<select
              aria-label="Font size"
              onchange={(e) => style("font.size", e.currentTarget.value)}
              >{#each [10, 11, 12, 14, 16, 18, 24, 32] as n}<option
                  selected={n === 12}>{n}</option
                >{/each}</select
            ><button onclick={() => style("alignment.horizontal", "left")}
              >⇤</button
            ><button onclick={() => style("alignment.horizontal", "center")}
              >↔</button
            ><button onclick={() => style("alignment.horizontal", "right")}
              >⇥</button
            >{/if}
        </div>
        <div class="tool-group">
          <button class="accent" onclick={insertFunction}
            >ƒ Add a calculation</button
          ><button onclick={explain}>Explain cell</button>
        </div>
      </div>
      <div class="formula-bar">
        <form
          onsubmit={(e) => {
            e.preventDefault();
            void go();
          }}
        >
          <input aria-label="Cell address" bind:value={location} />
        </form>
        <span>ƒx</span><input
          aria-label="Formula or cell contents"
          placeholder="Select a cell, then type a value or formula…"
          bind:value={input}
          oninput={() => (inputDirty = true)}
          onkeydown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void commit();
            }
          }}
        /><button onclick={commit} title="Apply value">✓</button><small
          >{multi.length
            ? `${multi.length} separate references selected`
            : rangeText}</small
        >
      </div>
      {#if doc.metadata.business}<div class="sheet-notice">
          Business worksheets are generated from your ledger. <button
            onclick={() => (page = "business")}
            >Open Business workspace →</button
          >
        </div>{/if}
      <div
        class="grid-scroll"
        bind:this={grid}
        onscroll={scroll}
        role="grid"
        aria-label="Spreadsheet"
        tabindex="0"
      >
        <div
          class="grid-space"
          style:width={`${offset(Math.min(16384, maxCol) + 1, true) + 48}px`}
          style:height={`${offset(Math.min(1048576, maxRow) + 1, false) + 30}px`}
        >
          {#each cells as c}<div
              role="gridcell"
              tabindex="-1"
              class="cell"
              class:selected={selected(c.r, c.c)}
              class:cell-error={c.text.startsWith("#")}
              style={cellStyle(c)}
              onclick={(e) => void selectCell(c.r, c.c, e)}
              ondblclick={() => startEdit(c)}
              onkeydown={() => {}}
              title={c.input.startsWith("=") ? c.input : c.text}
            >
              {#if editing && c.r === row && c.c === column}<input
                  use:focusEditor
                  aria-label="Edit cell"
                  bind:value={input}
                  oninput={() => (inputDirty = true)}
                  onkeydown={(e) => {
                    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") return;
                    e.stopPropagation();
                    if (e.key === "Enter") {
                      e.preventDefault();
                      void commit();
                    }
                    if (e.key === "Escape") {
                      editing = false;
                      input = c.input;
                      inputDirty = false;
                    }
                  }}
                  onblur={() => {
                    if (editing) void commit();
                  }}
                />{:else}{c.text}{/if}
            </div>{/each}
          {#if layout !== "Endless"}{#each Array.from({ length: 5 }, (_, i) => i + 1) as n}<div
                class="page-boundary"
                style:left={`${48 + n * 8 * columnWidth}px`}
                style:top={`${scrollTop}px`}
                style:height="100%"
              ></div>{/each}{/if}
          {#each visibleCols as c}<div
              class="column-header"
              style:left={`${48 + offset(c, true)}px`}
              style:top={`${scrollTop}px`}
              style:width={`${sizeAt(c, true)}px`}
            >
              {col(c)}
            </div>{/each}{#each visibleRows as r}<div
              class="row-header"
              style:top={`${30 + offset(r, false)}px`}
              style:left={`${scrollLeft}px`}
              style:height={`${sizeAt(r, false)}px`}
            >
              {r}
            </div>{/each}
          <div
            class="corner"
            style:left={`${scrollLeft}px`}
            style:top={`${scrollTop}px`}
          >
            ▦
          </div>
        </div>
      </div>
      <div class="sheet-tabs">
        <button title="Add worksheet" onclick={() => act("addSheet")}>＋</button
        >{#each doc.sheets as s, i}{#if s.state === "visible"}<button
              class:active={sheet === i}
              onclick={() => {
                sheet = i;
                row = column = endRow = endCol = 1;
                input = "";
                inputDirty = false;
                void loadGrid(active, i, firstRow, firstCol);
              }}
              ondblclick={() =>
                showInput("Rename worksheet", "renameSheet", s.name)}
              >{s.name}</button
            >{/if}{/each}
      </div>
    {/if}
    <footer>
      <span>{status}</span><span class="creator"
        >Created by Forbidden_Fantasy</span
      ><label
        >Zoom <input
          type="range"
          min="60"
          max="160"
          step="10"
          bind:value={zoom}
          onchange={prefs}
        />{zoom}%</label
      >
    </footer>
  </main>
</div>
{#if errorText}<div class="toast" role="alert">
    <div>
      <strong>We couldn’t complete that action</strong>
      <p>{errorText}</p>
    </div>
    <button onclick={() => (errorText = "")} aria-label="Dismiss error"
      >✕</button
    >
  </div>{/if}
{#if modal}<div class="modal-backdrop">
    <section
      class="modal"
      class:wide={["chooseWorkbook","customSetup","customDetails","wizard", "functions", "rules", "chart", "addSheets"].includes(modal)}
    >
      <header>
        <h2>
          {{
            chooseWorkbook: "New workbook",
            customSetup: "Build a custom workbook",
            customDetails: "Set up linked worksheets",
            addSheets: "Add worksheets to this workbook",
            wizard: "Set up your business workbook",
            functions: "Make a calculation",
            settings: "Make yourself at home",
            rules: "Official rules & research",
            help: "A little help",
            limitations: "Current feature support",
            input: genericTitle,
            explain: "Explain this cell",
            chart: "Chart preview",
            print: "Print your worksheet",
          }[modal]}
        </h2>
        <button aria-label="Close dialog" onclick={() => (modal = "")}>✕</button
        >
      </header>
      {#if modal === "chooseWorkbook"}
        <TemplateChooser {templates} choose={newWorkbook} />
      {:else if modal === "customSetup"}
        <form onsubmit={e => {
          e.preventDefault();
          if (usesBusinessSheets) { business.name ||= customTemplate.name; modal = "customDetails"; }
          else void finishWizard();
        }}>
          <TemplateBuilder bind:value={customTemplate} catalog={templateCatalog} />
          {#if selectedWorksheets.length}<p>Selected worksheets · {selectedWorksheets.map(s => s.name).join(", ")}</p>{/if}
          <div class="panel-tools"><button type="button" onclick={() => modal = "chooseWorkbook"}>Back to templates</button>
            <button class="primary" disabled={creating || !selectedWorksheets.length || !customTemplate.name?.trim()}>{usesBusinessSheets ? "Continue" : "Create workbook"}</button></div>
        </form>
      {:else if modal === "customDetails"}
        <form onsubmit={e => { e.preventDefault(); void finishWizard(); }}>
          <div class="form-grid">
            <label>Workbook name<input required bind:value={business.name} /></label>
            <label>Country<input required list="custom-countries" bind:value={business.country} /></label>
            <datalist id="custom-countries"><option>South Africa</option><option>United Kingdom</option><option>United States</option></datalist>
            <label>Currency code<input required maxlength="3" pattern={"[A-Za-z]{3}"} bind:value={business.currency} /></label>
            <label>Workbook month<input type="month" required bind:value={business.period} /></label>
            <label>Financial year starts in month<input required type="number" min="1" max="12" bind:value={business.year_start} /></label>
          </div>
          <details class="entry-options"><summary>More business details</summary>
            <div class="form-grid">
              <label>Business sector<select bind:value={business.kind}><option>General</option><option>Retail</option><option>Service</option><option>Security</option></select></label>
              <label>State / province / region<input bind:value={business.region} /></label>
              <label>Registration details<input bind:value={business.registration} /></label>
              <label>Tax registration<input bind:value={business.tax_registration} /></label>
            </div>
          </details>
          <div class="panel-tools"><button type="button" onclick={() => modal = "customSetup"}>Back to worksheets</button>
            <button class="primary" disabled={creating || !business.name.trim()}>Create workbook</button></div>
        </form>
      {:else if modal === "addSheets"}<form onsubmit={e=>{e.preventDefault();attachSheets();}}>{#if !doc.metadata.business && usesBusinessSheets}<div class="form-grid"><label>Workbook name<input required bind:value={business.name} /></label><label>Country<input required bind:value={business.country} /></label><label>Currency code<input required maxlength="3" bind:value={business.currency} /></label><label>Month<input required type="month" bind:value={business.period} /></label></div>{/if}<TemplateBuilder bind:value={customTemplate} catalog={templateCatalog} locked={doc.sheets.map((s:any)=>s.name)} /><button class="primary">Add selected worksheets</button></form>
      {:else if modal === "settings"}<label
          >Theme<select bind:value={theme} onchange={prefs}
            ><option value="system">System</option><option value="light"
              >Light</option
            ><option value="dark">Dark</option><option value="plum">Signature plum</option></select
          ></label
        ><label
          >Interface mode<select bind:value={mode} onchange={prefs}
            ><option>Simple</option><option>Advanced</option></select
          ></label
        ><label
          >Interface scale<input
            type="range"
            min="85"
            max="140"
            step="5"
            bind:value={scale}
            onchange={prefs}
          />{scale}%</label
        >
        <p>
          Simple mode keeps familiar tools within reach. Both modes work with
          the same workbook data.
        </p>
        <button onclick={() => licenseDialog.showModal()}>Application license</button>
      {:else if modal === "wizard"}<div class="wizard-steps">
          {#each ["Company", "Business", "Accounting", "Payroll", "Contributions", "Worksheets", "Review"] as s, i}<button
              class:active={i === wizardStep}
              onclick={() => (wizardStep = i)}>{i + 1}. {s}</button
            >{/each}
        </div>
        {#if wizardStep === 0}<div class="form-grid">
            <label
              >Company name<input required bind:value={business.name} /></label
            ><label
              >Country<select bind:value={business.country}
                >{#each ["South Africa", "United States", "United Kingdom", "Austria", "Belgium", "Bulgaria", "Croatia", "Cyprus", "Czechia", "Denmark", "Estonia", "Finland", "France", "Germany", "Greece", "Hungary", "Ireland", "Italy", "Latvia", "Lithuania", "Luxembourg", "Malta", "Netherlands", "Poland", "Portugal", "Romania", "Slovakia", "Slovenia", "Spain", "Sweden", "Canada", "Australia", "New Zealand", "India", "Japan", "China", "Brazil", "Mexico", "Namibia", "Botswana", "Zimbabwe", "Kenya", "Nigeria", "Other"] as c}<option
                    >{c}</option
                  >{/each}</select
              ></label
            ><label
              >State / province / region<input
                bind:value={business.region}
              /></label
            ><label
              >Currency code<input
                maxlength="3"
                bind:value={business.currency}
              /></label
            ><label
              >Registration details<input
                bind:value={business.registration}
              /></label
            ><label
              >Tax registration (optional)<input
                bind:value={business.tax_registration}
              /></label
            >
          </div>
        {:else if wizardStep === 1}<label
            >Business sector<select bind:value={business.kind}
              ><option>Security</option><option>Retail</option><option
                >Service</option
              ><option>General</option></select
            ></label
          >
          <p>
            All business types receive a linked ledger, payroll inputs,
            inventory and outstanding-balance worksheets. Sector-specific legal
            automation is not yet supported.
          </p>
        {:else if wizardStep === 2}<div class="form-grid">
            <label
              >Workbook month<input
                type="month"
                bind:value={business.period}
              /></label
            ><label
              >Financial year begins in month<input
                type="number"
                min="1"
                max="12"
                bind:value={business.year_start}
              /></label
            >
          </div>
          <p class="notice">
            Reporting uses an accrual ledger with a separate cash-movement view.
            Tax, VAT eligibility and contribution liabilities must be reviewed
            and entered manually. No unverified tax rates are applied.
          </p>
        {:else if wizardStep === 3}<p>
            Add hourly, daily or monthly employees after setup. Attendance
            calendars include paid hours, overtime, additional earnings and
            explicit deductions. Unmarked days do not reduce fixed monthly
            salaries.
          </p>
          <p>
            Each employee has individual contractual rates and a manual tax
            amount. Overnight shifts must currently be split into dated entries.
          </p>
        {:else if wizardStep === 4}<p>
            Custom employee and employer contributions are configured per
            employee. No mandatory scheme is silently assumed or disabled.
            Statutory eligibility and rates require professional review.
          </p>
          <button onclick={() => (modal = "rules")}
            >Read verified source research</button
          >
        {:else if wizardStep === 5}{#if chosenTemplate === "Custom Template"}<TemplateBuilder bind:value={customTemplate} catalog={templateCatalog} />{:else}<p>
            The workbook includes these linked reporting worksheets:
          </p>
          <div class="chips">
            {#each ["Overview", "Income", "Expenses", "Payroll", "Taxes and Contributions", "Profit and Loss", "Assets", "Refunds", "Inventory", "Settings", "Outstanding Balances", "Payment History"] as s}<span
                >{s}</span
              >{/each}
          </div>
          <p>
            The ledger feeds financial summaries. Inventory is an independent
            calculation sheet.
          </p>
          {/if}
        {:else}<h3>{business.name || "Your company"} · {business.period}</h3>
          <p>
            {business.kind} · {business.country} / {business.region} · {business.currency}
          </p>
          <p>Company identity: {business.company_id}</p>
          <p>
            Financial year starts in month {business.year_start}. Ledger method:
            accrual. All tax and statutory payroll amounts require manual
            review.
          </p>
          <p>
            Historical posted records cannot be silently overwritten. Use
            payment reversals for corrections. New months retain company
            identity for rollover.
          </p>{/if}
        <div class="dialog-actions">
          <button disabled={wizardStep === 0} onclick={() => wizardStep--}
            >Back</button
          >{#if wizardStep < 6}<button
              class="primary"
              onclick={() => wizardStep++}>Continue →</button
            >{:else}<button
              class="primary"
              disabled={!business.name || business.currency.length !== 3}
              onclick={finishWizard}>Create workbook</button
            >{/if}
        </div>
      {:else if modal === "functions"}<div class="function-layout">
          <div>
            <input
              placeholder="Try “add numbers” or “highest amount”…"
              bind:value={functionSearch}
            />
            <div class="function-list">
              {#each functions.filter((f) => (f.name + " " + f.description)
                  .toLowerCase()
                  .includes(functionSearch.toLowerCase())) as f}<button
                  class:active={f.name === functionChosen.name}
                  onclick={() => chooseFunction(f)}
                  ><strong>{f.name}</strong><small>{f.description}</small
                  ></button
                >{/each}
            </div>
          </div>
          <div>
            <h3>{functionChosen.name}</h3>
            <p>{functionChosen.description}</p>
            <p><b>Inputs:</b> {functionChosen.inputs}</p>
            <p>
              <code>={functionChosen.example}</code><small
                >{functionChosen.result}</small
              >
            </p>
            <label>Formula<textarea bind:value={formula}></textarea></label
            ><label
              >Output cell on {sheetName}<input bind:value={output} /></label
            >
            <p>
              Use Shift-click for a range, or Ctrl-click separate cells before
              opening this assistant. Quoted sheet references can be edited
              here.
            </p>
            <button onclick={formulaPreview}>Preview result</button
            >{#if preview}<strong class="preview-value">{preview}</strong>{/if}
            <div class="dialog-actions">
              <button class="primary" onclick={insertMapped}
                >Insert calculation</button
              >
            </div>
          </div>
        </div>
      {:else if modal === "input"}{#if genericAction === "find"}<label
            >Find<input bind:value={findText} /></label
          >{/if}<label
          >{genericAction === "find" ? "Replace with" : "Value"}<input
            bind:value={genericValue}
            onkeydown={(e) => {
              if (e.key === "Enter") void runGeneric();
            }}
          /></label
        ><button
          class="primary"
          onclick={() => {
            if (genericAction === "format") {
              style("num_fmt", genericValue);
              modal = "";
            } else void runGeneric();
          }}>Apply to {rangeText}</button
        >
      {:else if modal === "explain"}<p><b>{sheetName}!{location}</b></p>
        <pre>{formula || "(Empty cell)"}</pre>
        <p>Calculated value: <strong>{selectedCell?.text || input}</strong></p>
        <p>
          {formula.startsWith("=")
            ? "This is a formula. Cell references identify its inputs; changing those cells recalculates dependent results."
            : "This cell contains a directly entered value."}
        </p>
        <p>
          References found: {formula
            .match(
              /(?:'[^']+'!|[A-Za-z_]+!)?\$?[A-Z]{1,3}\$?\d+(?::\$?[A-Z]{1,3}\$?\d+)?/g,
            )
            ?.join(", ") || "None"}
        </p>
        <p>
          Common errors: #DIV/0! means division by zero; #REF! means a missing
          reference; #NAME? means an unknown function or name. Detailed
          intermediate traces are not yet available.
        </p>
      {:else if modal === "rules"}<p class="notice">
          Source-verified research is not a complete payroll or tax filing
          engine. The application does not automatically apply these statutory
          rates.
        </p>
        {#each rules.rules as rule}<article class="rule">
            <h3>{rule.id}</h3>
            <p>{rule.kind} · {rule.effectiveFrom} to {rule.effectiveTo}</p>
            {#each rule.sources as source}<a
                href={source}
                target="_blank"
                rel="noreferrer">{source}</a
              >{/each}{#if "limitations" in rule}<p>
                {rule.limitations?.join(" ")}
              </p>{/if}
          </article>{/each}
      {:else if modal === "help"}<p>
          Click a cell to select it. Double-click or type to edit. Enter applies
          a value. Shift-click extends a range. Ctrl-click collects separate
          formula references.
        </p>
        <table>
          <tbody
            >{#each [["Ctrl O", "Open workbook"], ["Ctrl S", "Save workbook"], ["Ctrl Shift S", "Save As"], ["Ctrl C / X / V", "Copy / cut / paste cell contents"], ["Ctrl Z / Y", "Undo / redo"], ["F2", "Edit selected cell"], ["Arrow keys / Tab", "Move between cells"], ["Delete", "Clear selected contents"]] as pair}<tr
                ><td><kbd>{pair[0]}</kbd></td><td>{pair[1]}</td></tr
              >{/each}</tbody
          >
        </table>
        <p>
          Business worksheets are projections of the Business workspace. Record
          transactions and payments there to retain audit history.
        </p>
      {:else if modal === "limitations"}<p>
          This build is an incomplete implementation of the full project brief.
          Supported: core cell editing, IronCalc formula calculations, XLSX
          cells and styles, basic ODS cells/formulas, templates, contractual
          payroll inputs, ledger payments, reconciliation and local recovery.
        </p>
        <p>
          Not yet complete: charts saved into workbook files, pivot tables,
          structured tables, advanced filters, drawing objects, comprehensive
          protection, validation controls, statutory payroll automation,
          security-site costing, full inventory accounting, bounded page
          editing, full print-layout options, and advanced external-file
          preservation.
        </p>
        <p>
          ODS style preservation is not implemented. Never overwrite an
          important complex original without retaining a copy.
        </p>
      {:else if modal === "chart"}<label
          >Chart type<select bind:value={chartType}
            ><option>Column</option><option>Bar</option><option>Line</option
            ><option>Area</option><option>Scatter</option></select
          ></label
        >
        <p>
          Live preview from the selection’s first two columns. Charts are not
          yet embedded in saved workbooks.
        </p>
        {@const max = Math.max(
          1,
          ...chartData.map((d) => Math.abs(d.value)),
        )}<svg
          class="chart"
          viewBox="0 0 700 320"
          role="img"
          aria-label={`${chartType} chart`}
          ><line
            x1="40"
            y1="270"
            x2="680"
            y2="270"
            stroke="currentColor"
          />{#if chartType === "Line" || chartType === "Area"}<polyline
              fill={chartType === "Area" ? "#a478d833" : "none"}
              stroke="#b58af0"
              stroke-width="3"
              points={chartData
                .map(
                  (d, i) =>
                    `${50 + (i * 600) / Math.max(1, chartData.length - 1)},${260 - (d.value / max) * 210}`,
                )
                .join(" ")}
            />{/if}{#each chartData as d, i}{@const x =
              50 +
              (i * 600) /
                Math.max(1, chartData.length)}{#if chartType === "Column"}<rect
                {x}
                y={260 - (Math.max(0, d.value) / max) * 210}
                width={500 / Math.max(1, chartData.length)}
                height={(Math.abs(d.value) / max) * 210}
                rx="3"
                fill="#b58af0"
              />{:else if chartType === "Bar"}<rect
                x="100"
                y={15 + (i * 270) / Math.max(1, chartData.length)}
                width={(Math.max(0, d.value) / max) * 500}
                height={220 / Math.max(1, chartData.length)}
                rx="3"
                fill="#b58af0"
              />{:else}<circle
                cx={x}
                cy={260 - (d.value / max) * 210}
                r="4"
                fill="#b58af0"
              />{/if}{#if chartType !== "Bar"}<text
                {x}
                y="295"
                fill="currentColor"
                font-size="10">{d.label.slice(0, 12)}</text
              >{/if}{/each}</svg
        >
      {:else if modal === "print"}<div class="form-grid">
          <label
            >Rows from row 1<input
              type="number"
              min="1"
              max="10000"
              bind:value={printRows}
            /></label
          ><label
            >Columns from column A<input
              type="number"
              min="1"
              max="100"
              bind:value={printCols}
            /></label
          ><label
            >Paper<select bind:value={paper}
              ><option>A4</option><option>A3</option><option>Letter</option
              ><option>Legal</option></select
            ></label
          ><label
            >Orientation<select bind:value={orientation}
              ><option>landscape</option><option>portrait</option></select
            ></label
          >
        </div>
        <p>
          Columns fit the page width; rows continue onto additional pages. Large
          column selections may become hard to read.
        </p>
        <button class="primary" onclick={printPreview}
          >Preview selected worksheet</button
        >{/if}
    </section>
  </div>{/if}
{#if printing}<div class="print-preview">
    <div class="print-controls">
      <button onclick={() => (printing = false)}>← Back to workbook</button
      ><button class="primary" onclick={nativePrint}
        >Print using system dialog</button
      >
    </div>
    <div class="paper">
      <h2>{printTitle}</h2>
      <table>
        <tbody
          >{#each printRowsData as r}<tr
              >{#each r as c}<td>{c}</td>{/each}</tr
            >{/each}</tbody
        >
      </table>
    </div>
  </div>{/if}
