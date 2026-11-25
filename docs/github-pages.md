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
