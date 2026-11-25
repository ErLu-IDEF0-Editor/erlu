#!/usr/bin/env python3
"""Generate a deployment-specific offline asset list and cache key."""
import hashlib,json,os,pathlib
root=pathlib.Path(os.environ.get('TRUNK_STAGING_DIR','dist/web'))
files=sorted(p for p in root.rglob('*') if p.is_file() and p.name not in {'sw.js','precache.json'})
entries=['./']+['./'+p.relative_to(root).as_posix() for p in files]
if (root/'version.json').exists():
 version=json.loads((root/'version.json').read_text())['version']
 entries += [f'./startup.js?v={version}',f'./runtime.js?v={version}']
version=hashlib.sha256(b''.join(hashlib.sha256(p.read_bytes()).digest() for p in files)).hexdigest()[:16]
(root/'precache.json').write_text(json.dumps(entries))
p=root/'sw.js';p.write_text(p.read_text().replace("erlu-0.1.0",f'erlu-{version}'))
