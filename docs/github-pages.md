# GitHub Pages

The multilingual project homepage is published at <https://erlu-idef0-editor.github.io/erlu/>. The **Try online** button opens the editor at <https://erlu-idef0-editor.github.io/erlu/editor.html>.

The homepage supports English, Russian, French, Spanish, Arabic (RTL), and Simplified Chinese. It uses the browser language initially, remembers an explicit selection locally, and supports shareable `?lang=ru` links. Opening the editor from the homepage also selects that interface language.

In repository **Settings → Pages → Build and deployment**, select **GitHub Actions** as the source. Publishing from a branch would serve repository documentation instead of the built editor.

The `GitHub Pages` workflow builds the editor and its three bundled plugins, runs Chromium acceptance tests under the repository subpath, and publishes only `dist/web`. Pushes to `master` deploy automatically. The workflow can also be started manually; only `master` deploys.

The Trunk post-build hook `scripts/build_site.py` moves the compiled editor to `editor.html`, copies the static homepage from `web/site`, then generates the offline asset list for both pages. The installed PWA opens the editor directly.

The deployment uses relative asset, plugin, manifest, and service worker paths. Documents remain in the browser; GitHub Pages hosts static application files.

To check a local release build under the same path:

```sh
npm --prefix web ci
npm --prefix web run plugins
trunk build --release --locked
ERLU_BASE_PATH=/erlu/ npm --prefix web test -- --project chromium
```

Install Playwright Chromium first with `cd web && npx playwright install chromium`, or set `CHROMIUM_PATH` to an installed Chromium executable.

## Startup and updates

The web editor shows WASM download progress, startup errors with a retry button, and a visible selector for all six UN languages. The desktop and web applications also have a dedicated Language menu.

The post-build step replaces Trunk’s implicit initializer with `web/startup.js`. Runtime and startup requests are tied to a build version to avoid mixing cached adapters with newer WASM. Navigations use the network when online and fall back to the offline cache. Failed module downloads and unavailable WebGL are reported instead of leaving a blank page.

**Check for updates** checks the deployed version and offers an explicit reload. Unsaved changes require confirmation. The service worker never reloads an editing tab automatically. For installations cached by older releases, one hard reload (Ctrl+Shift+R in Firefox on Linux) loads the new updater; this does not erase IndexedDB document recovery.
