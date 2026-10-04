#!/usr/bin/env python3
"""Validate the app's report while retaining unrelated desktop startup logs."""
import json
import pathlib
import sys

reports = []
for line in pathlib.Path(sys.argv[1]).read_text().splitlines():
    try:
        value = json.loads(line)
    except ValueError:
        continue
    if isinstance(value, dict) and 'api_version' in value:
        reports.append(value)
if len(reports) != 1:
    sys.exit('Expected one application diagnostic report')
v = reports[0]
assert v['api_version'] == 1 and v['profile_count'] == 13 and v['bundle_count'] == 16
assert v['preview_bytes'] > 0 and v['profile_preview_bytes'] > 0
if '--flatpak' in sys.argv:
    assert v['host']['flatpak'] is True
print('Packaged Rust API, catalogs, previews and host diagnostics passed')
