// Run after npm run build. Set BROWSER_EXECUTABLE to a Chromium-family browser.
const assert = require("node:assert/strict");
const fs = require("node:fs/promises");
const path = require("node:path");
const { chromium } = require("playwright-core");

async function run(browser) {
  const dist = path.resolve(__dirname, "../dist");
  const page = await browser.newPage({ viewport: { width: 960, height: 640 }, colorScheme: "light" });
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  page.setDefaultTimeout(6000);
  await page.route("http://studio.test/**", async route => {
    const pathname = new URL(route.request().url()).pathname;
    const filename = path.join(dist, pathname === "/" ? "index.html" : pathname);
    await route.fulfill({ path: filename });
  });
  await page.addInitScript(() => {
    const fixture = window.testUpdate = { calls: [], next: null, install: "installed", failSave: false };
    const workbook = { id: "sample", name: "Sample", metadata: {}, sheets: [{ name: "Sheet1", state: "visible" }] };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
      transformCallback: () => 1,
      invoke: async (cmd, args) => {
        fixture.calls.push({ cmd, args });
        switch (cmd) {
          case "startup": return { templates: [], settings: {}, recent: [], recovery: [], launchFiles: [] };
          case "check_for_updates":
            if (fixture.next === "offline") throw "Unable to connect to GitHub.";
            if (fixture.next === "pending") return new Promise(resolve => { fixture.resolve = resolve; });
            return fixture.next;
          case "install_update": return fixture.install;
          case "create_workbook": return { ...workbook, dirty: false };
          case "action": return { ...workbook, dirty: true };
          case "viewport": return { cells: [], merges: [], columnSizes: [], rowSizes: [], rows: 50, cols: 20 };
          case "save_workbook":
            if (fixture.failSave) throw "Could not save workbook.";
            return workbook;
          default: return null;
        }
      },
    };
  });
  await page.clock.install();
  await page.goto("http://studio.test/");
  await page.waitForFunction(() => window.testUpdate.calls.some(call => call.cmd === "show_ready_window"));
  const toolbar = page.locator(".top-actions");
  const check = toolbar.getByRole("button", { name: "Check For Updates", exact: true });
  const dialog = page.getByRole("dialog", { name: "Software update" });
  const close = dialog.getByRole("button", { name: "Close", exact: true });
  const calls = () => page.evaluate(() => window.testUpdate.calls.map(call => call.cmd));
  assert.deepEqual((await toolbar.getByRole("button").allTextContents()).map(s => s.trim()).slice(-3),
    ["+ New workbook", "Check For Updates", "Support Me"]);
  assert.equal(await page.locator("html").getAttribute("data-theme"), "light");
  await page.emulateMedia({ colorScheme: "dark" });
  await page.waitForFunction(() => document.documentElement.dataset.theme === "dark");
  await page.clock.fastForward("05:00");
  assert(!(await calls()).includes("check_for_updates"));

  await check.click();
  await dialog.getByText("You’re up to date.").waitFor();
  await close.click();
  await page.evaluate(() => { window.testUpdate.next = "offline"; });
  await check.click();
  await dialog.getByText(/Unable to connect to GitHub/).waitFor();
  await close.click();

  await page.evaluate(() => { window.testUpdate.next = "pending"; });
  await check.click();
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await page.evaluate(() => window.testUpdate.resolve("1.0.1"));
  await page.waitForFunction(() => !document.querySelector('[aria-haspopup="dialog"]').disabled);
  assert(!(await calls()).includes("install_update"));

  await page.getByRole("button", { name: "＋ Blank workbook", exact: true }).click();
  await page.getByLabel("Formula or cell contents").fill("Pending edit");
  await page.evaluate(() => { window.testUpdate.next = "1.0.1"; window.testUpdate.failSave = true; });
  await check.click();
  await dialog.getByText(/Could not save workbook/).waitFor();
  assert(!(await calls()).includes("install_update"));
  assert.equal(await page.evaluate(() => window.testUpdate.calls.find(c => c.cmd === "action").args.args.value), "Pending edit");
  await close.click();

  await page.evaluate(() => { window.testUpdate.failSave = false; });
  await check.click();
  await dialog.getByText(/V1\.0\.1 is installed/).waitFor();
  assert.deepEqual((await calls()).slice(-3), ["check_for_updates", "save_workbook", "install_update"]);
  assert.deepEqual(await page.evaluate(() => window.testUpdate.calls.find(c => c.cmd === "install_update").args), { version: "1.0.1" });
  await close.click();
  await page.evaluate(() => { window.testUpdate.install = "installer-started"; });
  await check.click();
  await page.waitForFunction(() => window.testUpdate.calls.some(c => c.cmd === "plugin:window|close"));
  assert.deepEqual(errors, []);
  console.log("PASS manual checks, release feedback, cancelled check, unsaved edit, failed save, versioned install, Windows handoff, toolbar order, and system theme.");
}

(async () => {
  await fs.access(path.join(__dirname, "../dist/index.html"));
  const browser = await chromium.launch({ executablePath: process.env.BROWSER_EXECUTABLE });
  try { await run(browser); } finally { await browser.close(); }
})().catch(error => { console.error(error.message); process.exitCode = 1; });
