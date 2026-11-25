# Browser tests and CI

The web job builds the static application and runs the same scenarios in Chromium,
Firefox and WebKit. Browser-specific launch options live in each Playwright project;
Chromium flags must not be passed to Firefox or WebKit.

On Linux CI, Firefox runs with a virtual display and software WebGL. Xvfb uses 96 DPI,
and Firefox's native pixel scale is fixed at 1 to match the emulated device scale.
Chromium and WebKit run headless. One worker avoids contention between software GPU
contexts. The GUI arrow test uses the actual canvas bounds and verifies that its
rendering buffer matches CSS dimensions before computing pointer coordinates.

To reproduce the CI setup on a Linux system with Playwright dependencies:

```sh
npm --prefix web ci
npm --prefix web run plugins
trunk build --release --locked
(cd web && npx playwright install --with-deps chromium firefox webkit)
xvfb-run -a --server-args="-screen 0 1600x1200x24 -dpi 96" \
  env CI=true ERLU_FIREFOX_HEADED=1 LIBGL_ALWAYS_SOFTWARE=1 npm --prefix web test
```

`CHROMIUM_PATH` selects a local Chromium executable and, by default, runs only that
project. Set `ERLU_ALL_BROWSERS=1` to retain the Firefox and WebKit projects when
using a custom Chromium. NixOS also needs Nix-compatible Playwright browser builds;
`PLAYWRIGHT_BROWSERS_PATH` can point to the matching Nix browser set.

The AI test grants access through the GUI, inserts the allowed origin as a single
text input, and waits for the connector's read-only `connectionStatus()` snapshot.
It then performs the actual SDK handshake, calls actions, disables access through
the GUI and verifies revocation. The status method cannot grant access.

Startup failures report the loading message, console errors and WebGL availability.
CI retains failure screenshots, traces and an HTML report in the
`erlu-browser-test-results` artifact (or `erlu-pages-browser-test-results` for Pages). No retries or skipped browser projects are used
to hide failures.

The currency test opens Edit → Properties → ABC Units and selects EUR, CAD and
conventional units with real mouse events. It locates egui controls through the
read-only `erluRuntime.uiSnapshot()` geometry snapshot instead of fixed screen
coordinates, waits for the draft symbol and dialog closure, and checks the stored
symbol and SVG output. The snapshot is rebuilt each frame and cannot edit the
application. Both desktop builds and the document/AI action API are unaffected.

The Pages job runs Chromium under the repository subpath. To reproduce that path:

```sh
ERLU_BASE_PATH=/erlu/ CI=true npm --prefix web test -- --project chromium
```

`FIREFOX_PATH` can select a Nix-compatible Firefox executable. Startup regression tests cover failed WASM downloads, all six language-menu entries, continued frame rendering, and online navigation with a stale editor in the offline cache.
