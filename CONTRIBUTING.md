# Contributing to Visper

Thanks for your interest. Issues, discussions and pull requests may be written in English or Chinese (欢迎使用中文).

## Before you start

- **Read the scope.** Visper is intentionally small. The list of what it does *not* do is in the [README](README.md#out-of-scope) and in the ADRs under [`docs/adr/`](docs/adr/). A feature outside that scope needs an issue and agreement first; please don't open a large PR for it unannounced.
- **Use the domain language.** [`GLOSSARY.md`](GLOSSARY.md) defines the terms used in code, UI text and docs (Recording, Transcription, Transcript, Segment, Library, and the statuses). Use those words, and avoid the ones listed under *Avoid*.
- **Respect the visual spec.** UI changes should follow [`docs/design/visual-spec-v1.md`](docs/design/visual-spec-v1.md).
- **Changing a decision?** If your change goes against an existing ADR, add a new ADR explaining why, in the same short style.

## Setting up

Follow [Build and run](README.md#build-and-run) in the README. Use the preview scenarios (`cargo run -- --preview <scenario>`) to check UI changes without touching your own library or downloading the model.

## Making a change

1. Fork the repository and create a branch from `main`.
2. Keep the change focused: one fix or feature per pull request.
3. Add or update tests. Rules about the library, queue and statuses belong in `src/library.rs`, which has no GUI dependency and is the easiest place to test. UI behaviour can be tested with GPUI's `TestAppContext`, as in `src/ui/render.rs`.
4. Before pushing, make sure these pass:

   ```bash
   cargo fmt --all --check
   ```

   ```bash
   cargo clippy --all-targets -- -D warnings
   ```

   ```bash
   cargo test
   ```

5. Add a line under **Unreleased** in [`CHANGELOG.md`](CHANGELOG.md) if users will notice the change.
6. Open a pull request and describe what changed and why. Screenshots help for anything visual; include both light and dark appearance when relevant.

## Code style

- Match the surrounding code: naming, comment density and idiom.
- Comments explain *why*, not *what*.
- Prefer small, deep modules with simple interfaces over many thin layers.
- New platform-specific code goes behind `cfg(target_os = ...)` so the Windows port stays possible (see [ADR 0001](docs/adr/0001-whisper-cpp-vulkan.md)).

## Reporting bugs

Open an issue with the bug report template. Include your macOS version, chip (Apple Silicon or Intel), how you built Visper, and the steps to reproduce. If a particular recording fails, say what format and length it is; please don't attach private recordings.

## License

By contributing, you agree that your contributions will be dual licensed under the MIT and Apache-2.0 licenses, as described in the [README](README.md#license).
