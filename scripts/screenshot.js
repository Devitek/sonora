#!/usr/bin/env gjs
// Render a URL in a real WebKitGTK WebView (same engine Tauri uses on Linux)
// with a transparent background and save a PNG snapshot. Used to capture the
// actual Sonora UI for the documentation site.
//
//   gjs scripts/screenshot.js <url> <out.png> [width] [height] [settleMs]

imports.gi.versions.Gtk = "3.0";
imports.gi.versions.Gdk = "3.0";
imports.gi.versions.WebKit2 = "4.1";
const { Gtk, Gdk, WebKit2, GLib } = imports.gi;

const url = ARGV[0];
const outPath = ARGV[1];
const W = parseInt(ARGV[2] || "640", 10);
const H = parseInt(ARGV[3] || "440", 10);
const settleMs = parseInt(ARGV[4] || "900", 10);

if (!url || !outPath) {
  printerr("usage: screenshot.js <url> <out.png> [w] [h] [settleMs]");
  imports.system.exit(2);
}

Gtk.init(null);

const win = new Gtk.Window({ type: Gtk.WindowType.TOPLEVEL });
win.set_default_size(W, H);
win.resize(W, H);

const view = new WebKit2.WebView();
view.set_size_request(W, H);

// Make the WebView background transparent so only the floating bar / panel show
// (the rest of the page is transparent by design) — lets us composite later.
try {
  const rgba = new Gdk.RGBA();
  rgba.parse("rgba(0,0,0,0)");
  view.set_background_color(rgba);
} catch (e) {
  printerr("warn: could not set transparent bg: " + e);
}

win.add(view);
win.show_all();

function snapshot() {
  view.get_snapshot(
    WebKit2.SnapshotRegion.FULL_DOCUMENT,
    WebKit2.SnapshotOptions.TRANSPARENT_BACKGROUND,
    null,
    (v, res) => {
      try {
        const surface = view.get_snapshot_finish(res);
        surface.flush();
        // Preferred: cairo surface -> PNG. Fall back to GdkPixbuf if needed.
        if (typeof surface.writeToPNG === "function") {
          surface.writeToPNG(outPath);
        } else {
          const w = surface.getWidth();
          const h = surface.getHeight();
          const pb = Gdk.pixbuf_get_from_surface(surface, 0, 0, w, h);
          pb.savev(outPath, "png", [], []);
        }
        print("wrote " + outPath);
      } catch (e) {
        printerr("snapshot error: " + e);
      }
      Gtk.main_quit();
    },
  );
}

let done = false;
view.connect("load-changed", (v, ev) => {
  if (ev !== WebKit2.LoadEvent.FINISHED || done) return;
  done = true;
  GLib.timeout_add(GLib.PRIORITY_DEFAULT, settleMs, () => {
    snapshot();
    return GLib.SOURCE_REMOVE;
  });
});

view.connect("load-failed", (v, ev, uri, err) => {
  printerr("load-failed: " + uri + " : " + err.message);
  Gtk.main_quit();
  return true;
});

view.load_uri(url);

GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 25, () => {
  printerr("timeout waiting for load");
  Gtk.main_quit();
  return GLib.SOURCE_REMOVE;
});

Gtk.main();
