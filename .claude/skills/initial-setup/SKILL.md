---
name: initial-setup
description: Sets up the development environment for GenVM repository. Use when setting up the repo for the first time or when dependencies need to be refreshed.
---

To set up the GenVM development environment:

1. **Enter the Nix flake environment:**
   ```bash
   nix develop .#full
   ```

2. **Initialize git submodules:**
   ```bash
   git submodule update --init --recursive --depth 1
   ```

3. **Source environment variables:**
   ```bash
   source env.sh
   ```
   This adds `tools/git-third-party` to PATH and sources `.env` if it exists.

4. **Update third-party dependencies:**
   ```bash
   ./tools/git-third-party/git-third-party update --all
   ```
   This updates wasmtime, wasm-tools, and applies GenVM-specific patches.

The repository will be ready for development with all dependencies properly configured.
