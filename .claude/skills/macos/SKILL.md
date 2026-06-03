---
name: macos
description: Read this BEFORE trying to fix a GenVM build that fails on macOS. GenVM is NOT built natively on macOS — building from source on Darwin breaks the deterministic runner-artifact invariant. Use a remote Linux nix-builder instead. Triggers on any build/nix/runner/linker/ar failure on a Mac (Apple Silicon or Intel).
---

# Building GenVM on macOS

## STOP — do not "fix" the build to run natively on macOS

If you are on macOS and a build (`nix build`, `ninja`, runner packaging, linking,
`ar`, FFI deps, …) fails, **do not** try to make GenVM compile natively on Darwin.
This has been attempted and **rejected** before. Native macOS builds break the
project's core invariant.

### Why native macOS builds are forbidden

Runner artifacts are content-addressed `.tar` files. **The file name encodes the
content hash** (a Nix `nix32` SHA-256), so two nodes that build the "same" runner
must produce byte-identical tars with identical names. This is what prevents
**determinism violations** across the network: one node running a correct artifact
and another running a differently-built artifact will disagree.

A native macOS build:

- produced **different and/or missing** runner tars vs. the Linux reference build
  (e.g. only one `cpython` tar instead of two, mismatched hashes), and
- silently changed the on-disk set of runners.

You can verify the reference set on Linux with:

```sh
find $(nix build .#runners-all --no-link --print-out-paths) -name '*.tar' \
  | sort | xargs sha256sum | sed -e 's+/nix/store/[^/]*++'
```

Any macOS output that does not reproduce these exact hashes is wrong.

### Specific anti-fixes — never do these

These are the tempting "fixes" that actually break determinism. Do **not** apply them:

1. **Removing `outputHash`** from fixed-output derivations (e.g.
   `runners/cpython/deps/ffi/default.nix`). The repo literally warns: *"the moment
   you delete this you should question yourself."* `outputHash` is what pins the
   artifact; deleting it destroys reproducibility.
2. **Adding a platform-native linker** path for Darwin.
3. **Patching `ar`** to work on macOS.
4. Any change that lets runners build on `aarch64-darwin` / `x86_64-darwin`
   instead of Linux.
5. **Editing `runners/support/versions/current.nix` hashes** to make a macOS
   build pass. A change whose only purpose is "make it build on macOS" must
   **not** touch those hashes — they pin the Linux reference artifacts. If a
   macOS build forces you to change a hash there, the build is producing the
   wrong bytes; fix the builder (use the Linux remote builder), not the hash.
6. **Removing / pruning old runner generations** (existing `.tar` entries or
   prior hash versions). A macOS-build-only change must leave the existing set
   of runner generations intact — silently dropping old ones is exactly the
   determinism-breaking regression that gets such changes rejected.

Per the maintainer: runners can only be built on Linux (x86_64). On macOS you
**download** prebuilt runners or **build them on a remote Linux builder** — you do
not build them locally.

## The supported path: a remote Linux nix-builder

Build Linux derivations on Linux, from your Mac, over SSH. The repo ships a
ready-made containerized Linux remote builder:

> **`support/macos/nix-builder/`** — see its `README.md` for the full, current
> procedure. Read that file; do not improvise.

In short (read the README for exact, up-to-date steps):

1. Install Rosetta (`softwareupdate --install-rosetta --agree-to-license`) and
   Docker Desktop with **"Use Rosetta for x86_64/amd64 emulation"** enabled.
2. `docker build -t nix-builder support/macos/nix-builder` (add
   `--platform linux/arm64` if needed), drop your `id_ed25519.pub` into the
   volume, and `docker run` it (privileged, port `2222`).
3. Register it in `/etc/nix/machines` and set `builders-use-substitutes = true`
   in `/etc/nix/nix.conf`. Add the host key to `known_hosts` **for root too**.
4. Sanity-check with
   `NIX_REMOTE="ssh-ng://root@localhost:2222" nix build --system x86_64-linux nixpkgs#hello`.

Notes that bite people (all in the README): use Nix `2.34.7`+ (older `2.2x`
mishandles `ssh-ng://host:port`), and the root user — not just your user — must
trust the builder's host key.

## If you just need to develop / run, not rebuild runners

You usually do not need to build runners at all. Download them instead (works on
any platform):

```bash
nix develop .#full --command python3 build/out/bin/post-install.py \
  --create-venv false --default-step false \
  --runners-download true --error-on-missing-executor false
```

See the `build` skill for the normal Rust build flow.
