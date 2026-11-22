## What this changes

<!-- The tool or fix, and the spec it follows (for example specifications/<tool>/...). -->

## How it was tested

<!-- Tests added, and anything checked by hand in the app. -->

## Checklist

- [ ] Every commit is signed off (`git commit -s`).
- [ ] Nothing here comes from reverse engineering Modo, and nothing is copied from Modo (code, icons, images, text) or from confidential Foundry material ([ground rules](../docs/CLEAN_ROOM.md)).
- [ ] Any adapted outside code is in its own module with its notice and licence, and new dependencies are in `docs/DEPENDENCIES.md`, all GPL-3.0-compatible.
- [ ] `cargo fmt --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings` and `cargo test --locked --workspace` pass.
- [ ] If this follows a spec, gaps are logged in `docs/SPECIFICATION_QUESTIONS.md` and the code is marked *provisional* where it guesses.
