#!/usr/bin/env gjs
// Headless WebKitGTK layout inspector.
//
// Loads a URL in a real WebKitGTK WebView (same engine Tauri uses on Linux) and
// dumps getBoundingClientRect + key computed styles for a set of selectors.
// Run via scripts/inspect-layout.sh (handles build + static server + Xvfb).
//
//   gjs scripts/inspect-layout.js <url> "<sel1,sel2,...>"

imports.gi.versions.Gtk = "3.0";
imports.gi.versions.WebKit2 = "4.1";
const { Gtk, WebKit2, GLib } = imports.gi;

const url = ARGV[0] || "http://localhost:8765";
const selectors = (ARGV[1] || ".big-mic,.mic,.empty,.body,.hud").split(",");
const clickSel = ARGV[2] || ""; // optional: click this before measuring

Gtk.init(null);

const win = new Gtk.Window({ type: Gtk.WindowType.TOPLEVEL });
win.set_default_size(460, 340);
win.resize(460, 340);
const view = new WebKit2.WebView();
view.set_size_request(460, 340);
win.add(view);
win.show_all();

function buildScript(sels) {
  return `(function(){
    const sels = ${JSON.stringify(sels)};
    const out = { __viewport: {
      innerWidth: window.innerWidth, innerHeight: window.innerHeight,
      dpr: window.devicePixelRatio,
      docClientWidth: document.documentElement.clientWidth,
      docClientHeight: document.documentElement.clientHeight,
    } };
    for (const s of sels) {
      const el = document.querySelector(s);
      if (!el) { out[s] = null; continue; }
      const r = el.getBoundingClientRect();
      const cs = getComputedStyle(el);
      out[s] = {
        tag: el.tagName,
        rect: { w: r.width, h: r.height, x: r.x, y: r.y },
        display: cs.display, position: cs.position,
        width: cs.width, height: cs.height,
        minWidth: cs.minWidth, minHeight: cs.minHeight,
        flex: cs.flex, alignSelf: cs.alignSelf,
        visibility: cs.visibility, transform: cs.transform,
        overflow: cs.overflow,
      };
    }
    return JSON.stringify(out, null, 2);
  })();`;
}

function measureAndQuit() {
  view.evaluate_javascript(buildScript(selectors), -1, null, null, null, (v2, res) => {
    try {
      print(view.evaluate_javascript_finish(res).to_string());
    } catch (e) {
      printerr("JS eval error: " + e);
    }
    Gtk.main_quit();
  });
}

let done = false;
view.connect("load-changed", (v, ev) => {
  if (ev !== WebKit2.LoadEvent.FINISHED || done) return;
  done = true;
  // Let layout settle, optionally click a control, settle again, then measure.
  GLib.timeout_add(GLib.PRIORITY_DEFAULT, 500, () => {
    if (clickSel) {
      const js = `document.querySelector(${JSON.stringify(clickSel)})?.click();`;
      view.evaluate_javascript(js, -1, null, null, null, () => {
        GLib.timeout_add(GLib.PRIORITY_DEFAULT, 500, () => {
          measureAndQuit();
          return GLib.SOURCE_REMOVE;
        });
      });
    } else {
      measureAndQuit();
    }
    return GLib.SOURCE_REMOVE;
  });
});

view.connect("load-failed", (v, ev, failingUri, err) => {
  printerr("load-failed: " + failingUri + " : " + err.message);
  Gtk.main_quit();
  return true;
});

view.load_uri(url);

GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 20, () => {
  printerr("timeout waiting for load");
  Gtk.main_quit();
  return GLib.SOURCE_REMOVE;
});

Gtk.main();
