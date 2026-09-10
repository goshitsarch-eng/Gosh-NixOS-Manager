# Historical: GTK4 → libcosmic migration

The files in this directory were written for the GUI rewrite from GTK4/libadwaita to libcosmic. They mix frozen contracts that landed, GTK-era paths that no longer exist (`window.rs`, `pages/`), and present-tense claims that are now false.

**Do not copy counts, toolkit names, crate layouts, or install instructions from these files into user-facing docs without re-reading the code.**

Current documentation:

- [../../README.md](../../README.md)
- [../architecture.md](../architecture.md)
- [../development.md](../development.md)
- [../reference.md](../reference.md)

| File | What it was |
|------|-------------|
| `PLAN.md` | Lead task list for the rewrite |
| `DECISIONS.md` | Numbered choices (C1–C22). Some are implemented; some (prefs migrate-once, APP_ID JSON path) are not |
| `architecture.md` | Target libcosmic architecture while GTK was still the GUI |
| `ux.md` | GTK page inventory and cosmic UX contract |
| `packaging.md` | Flatpak / CI / fake-helper plan |
| `review-phase1.md` | Pre-implementation review of the GTK tree |
| `TASKLOG.md` | Phase-2 diary |
| `REPORT.md` | Closest post-migration summary; still not a user README |

`progress.md` at the repo root is also historical (GTK bug-fix notes, old checkout paths).
