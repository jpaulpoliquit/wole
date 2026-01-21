# Status screen slow start investigation + spec

## Context
Opening the `wole status` TUI has a noticeable delay before the UI appears.

## Findings
* The status screen performs a blocking `gather_status` call before the TUI renders.
* On Windows, `gather_status` performs multiple WMI queries (process handle counts, page faults, top I/O processes, boot info), which are materially slower than sysinfo-only calls.
* The blocking call happens in both the CLI `status` command and when entering the status screen from the TUI dashboard.

## Goal
Make the status screen appear quickly, then fill in richer details asynchronously.

## Plan
1. Introduce a fast status gather path that skips WMI-heavy probes and uses sysinfo-only process data.
2. Use the fast gather path for initial status screen render (CLI `wole status` and TUI dashboard navigation).
3. Keep the existing async refresh behavior to populate full details after the screen is open.

## Out of scope
* Changing the data shown in the status screen once the full refresh completes.
* Altering JSON output behavior.
