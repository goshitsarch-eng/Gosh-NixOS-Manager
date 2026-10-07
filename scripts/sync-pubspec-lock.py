#!/usr/bin/env python3
"""Keep the Nix-readable lock snapshot identical to Flutter's checked-in lock."""
import json
import pathlib
import sys
import yaml

root = pathlib.Path(__file__).resolve().parent.parent
snapshot = root / 'desktop/pubspec.lock.json'
contents = json.dumps(yaml.safe_load((root / 'desktop/pubspec.lock').read_text()), indent=2, sort_keys=True) + '\n'
if '--check' in sys.argv:
    if not snapshot.exists() or snapshot.read_text() != contents:
        sys.exit('Run python3 scripts/sync-pubspec-lock.py after changing pubspec.lock')
else:
    snapshot.write_text(contents)
