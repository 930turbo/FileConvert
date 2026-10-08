# Project decisions

## Objective

Private, local, Windows-only-by-default conversion utility with the same practical right-click workflow as the reference project, but without using its application source tree.

## Optimization choices

1. No resident process. Explorer invokes `m5convert.exe` only after the user chooses a target.
2. No GPUI/Chromium/WebView UI. Errors use the native Windows message box.
3. Common image conversions stay in-process to avoid process startup overhead.
4. Optional heavyweight codecs are dynamically discovered and never loaded into Explorer.
5. The Explorer DLL asks the CLI only for a short target list and caches each extension for 60 seconds.
6. Release profile uses `opt-level=3`, fat LTO, one codegen unit, symbol stripping and abort-on-panic.
7. FFmpeg receives an explicit network protocol blacklist for local-only media processing.
8. Output is staged in a temporary directory next to the destination and then moved into place.

## Deliberately not carried over

Cloud jobs, paid API, billing, authentication, database, license verification, updater, website, browser extension, cross-platform integrations, release reproducibility infrastructure and analytics are outside this private build.
