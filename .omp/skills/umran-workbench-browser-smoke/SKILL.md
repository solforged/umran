---
name: umran-workbench-browser-smoke
description: "Use when smoke-testing the Umran browser workbench or refreshing README screenshots: shelf, world setup, stage, cards, export."
---

# Umran workbench browser smoke

1. `bun run build` runs the WASM build, `tsc --noEmit`, and vite. For a dev server, run `bun run dev` as a named service; it is ready on port 5173. While it runs, rebuild the WASM with `bun run build:wasm`, then reload.
   - If another session's unfinished Rust breaks the WASM build, make a clean copy with `git worktree add --detach /tmp/umran-wt HEAD` and copy only your own crate files into it. Then run `CARGO_TARGET_DIR=/tmp/umran-wt-target <repo>/node_modules/.bin/wasm-pack build crates/umran-web --target web --out-dir <repo>/web/src/wasm --out-name umran_web` from the worktree. wasm-pack is not on PATH. Check the frontend with `bun run check`. Remove the worktree afterwards.
2. Open headless Chrome for Testing with the `app` options from step 8 and `url: "http://127.0.0.1:5173/"`. `browser.open` takes an options object, not a URL string. A fresh `--user-data-dir` gives a true first run (shelf with two tiles). To show Sol a page, use the `tern-browser-and-preview` skill. If screenshots time out, the browser is stuck: run `browser.close({ name, kill: true })` and reopen.
3. Read text with `tab.evaluate("document.body.innerText")`; `tab.text()` needs a selector. Click by evaluating `[...document.querySelectorAll('button')].find(b => b.innerText...).click()`. Avoid long promises inside the page, which get collected; wait with Bun.sleep in the eval host. For colour scheme, use `page.emulateMediaFeatures([{name:'prefers-color-scheme',value:'light'}])` inside `tab.run`; there is no `emulateMedia`. `tab.evaluate` output is printed raw; never return a Buffer or screenshot bytes from it (use `tab.run` + `page.screenshot({ path })` and return a string).
4. Set `<select>` values with the prototype setter plus a change event, because React ignores plain assignment: `Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype,'value').set.call(s,v); s.dispatchEvent(new Event('change',{bubbles:true}))`. Pace and stop-for are no longer selects: open `.timebar button.play-more` ("How the years pass"), which shows `.popover-panel .pace-menu` with two radio fieldsets ("How quickly": slowly/steadily/swiftly; "Stop for": nothing/peoples meeting…/sound changes/anything at all). Click the radio inside the matching `label`, then dispatch Escape on `document` to close. To play without stopping, pick `nothing`; `swiftly` reached year ~3875 in 20 s. Toggle `.timebar button.play` to start and stop.
5. There is one look, the explorer's chart. `.light-switch select` takes `auto`, `light`, or `dark`, sets `data-theme` on `<html>`, and is saved under `umran.light`. Check both day and lamplight.
6. Flow:
   - A returning visitor reopens the last world on the stage. To reach setup, click the "Umran" brand, which opens the shelf, then "Unknown waters" (the new-world tile).
   - World setup (`.setup`):
     - A `.cartouche` on the chart holds `.sizes` link radios (small, middling, wide, vast) and "Redraw the coasts".
     - The `.setup-panel` book has a "read a chronicle already written" sample link, which opens year 4000.
     - It also has `.account` items and an "Another people" button (click it repeatedly to reach eight). Click `.account-name` to open one. The open `.account.chosen` has inline selects (NamingSelect, `aria-label="Way of life"`, `aria-label="Sounds"`) and the links "Hear other words" and "Adjust their sounds…".
     - Click a land to move the chosen people there.
     - The footer button `.begin` reads "Begin the chronicle".
  - Stage: `.cartouche` button opens the world menu (Other tellings, Field notebook, Export…, theme, Back to the shelf); `.map-tools` has Find and the layers Popover; `.pedia` cards with World/Chronicle buttons; map labels are `svg text[data-label-kind]`, click them with a dispatched MouseEvent; `.timebar` holds play, play-more, step-year, "Until something happens", marks, strike-out (undo).
   - Export sheet: `.appendix`, opened from the cartouche menu.
7. Verify:
   - `(await tab.errors()).entries` shows nothing beyond requestFailed.
   - Cards open from the map and the feed.
   - Playback speed is unchanged. Count years played in 6 s; it was 600 at "steadily".
8. Never use the `.word` class inside SVG map labels, because rem font sizes explode in map units; size lettering with `--label` in map units. Extra SVGs inside `.mapview` must override `.mapview svg {width/height:100%}` with a more specific selector. Harness `page.mouse.wheel` does not reach the map; dispatch a `WheelEvent` on the svg to test wheel zoom.
9. To test the Pages build or take README screenshots:
   - Use the app path `~/Library/Caches/ms-playwright/chromium-1234/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing` with args `["--headless=new", "--user-data-dir=/tmp/<profile>"]`, then `emulate({ viewport: { width: 1440, height: 900 } })`.
   - Save PNGs with `tab.run(async ({ page }, p) => page.screenshot({ path: p, type: 'png' }), { args: [path] })` into `docs/images/`. Close any open modal first and check `!document.querySelector('.modal')`.
   - Delete the temporary profile afterwards.
10. Close the tab when done. Test runs add worlds to the user's shelf (saved under `langgen.*` keys); mention them in the report.
