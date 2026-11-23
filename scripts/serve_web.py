#!/usr/bin/env python3
"""Serve the static build at a project subpath for browser acceptance tests."""
import http.server
import os
import pathlib
import re
import urllib.parse

ROOT = pathlib.Path(__file__).resolve().parent.parent / 'dist' / 'web'
PREFIX = os.environ.get('ERLU_BASE_PATH', '/')
if not re.fullmatch(r'/(?:[A-Za-z0-9_.-]+/)*', PREFIX):
    raise SystemExit('ERLU_BASE_PATH must be a slash-delimited path ending in /')


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def send_head(self):
        path = urllib.parse.urlsplit(self.path).path
        if PREFIX != '/' and path == PREFIX.rstrip('/'):
            self.send_response(301)
            self.send_header('Location', PREFIX)
            self.end_headers()
            return None
        if not path.startswith(PREFIX):
            self.send_error(404)
            return None
        original = self.path
        self.path = '/' + self.path[len(PREFIX):]
        try:
            return super().send_head()
        finally:
            self.path = original


if __name__ == '__main__':
    http.server.ThreadingHTTPServer(('127.0.0.1', 8080), Handler).serve_forever()
