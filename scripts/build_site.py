#!/usr/bin/env python3
"""Compose the project homepage and editor, then generate their offline cache."""
import os
import hashlib
import json
import re
from pathlib import Path
import runpy
import shutil

PROJECT = Path(__file__).resolve().parent.parent
root = Path(os.environ.get('TRUNK_STAGING_DIR', PROJECT / 'dist' / 'web'))
html = (root / 'index.html').read_text()
# Trunk's implicit initializer leaves failed imports/downloads on a permanent
# loading screen. Let our loader own download progress and startup failures.
pattern = r"<script type=\"module\">\s*import init, \* as bindings from '([^']+)';.*?module_or_path: '([^']+)'.*?</script>"
match = re.search(pattern, html, re.S)
if not match:
    raise SystemExit('Cannot locate Trunk WASM initializer')
version = hashlib.sha256((root / 'startup.js').read_bytes() + (root / 'runtime.js').read_bytes() + match[1].encode()).hexdigest()[:16]
assets = {'module':match[1], 'wasm':match[2], 'bytes':(root / match[2]).stat().st_size,
          'runtime':f'./runtime.js?v={version}', 'version':version}
html = html[:match.start()] + '<script type="application/json" id="erlu-assets">' + json.dumps(assets) + '</script>' + html[match.end():]
html = html.replace('src="./startup.js"', f'src="./startup.js?v={version}"')
(root / 'editor.html').write_text(html)
(root / 'version.json').write_text(json.dumps({'version':version}))
shutil.copytree(PROJECT / 'web' / 'site', root, dirs_exist_ok=True)
runpy.run_path(str(PROJECT / 'scripts' / 'precache.py'), run_name='__main__')
