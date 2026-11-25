#!/usr/bin/env python3
"""Compose the project homepage and editor, then generate their offline cache."""
import os
from pathlib import Path
import runpy
import shutil

PROJECT = Path(__file__).resolve().parent.parent
root = Path(os.environ.get('TRUNK_STAGING_DIR', PROJECT / 'dist' / 'web'))
(root / 'index.html').replace(root / 'editor.html')
shutil.copytree(PROJECT / 'web' / 'site', root, dirs_exist_ok=True)
runpy.run_path(str(PROJECT / 'scripts' / 'precache.py'), run_name='__main__')
