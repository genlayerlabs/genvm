---
name: build
description: Builds the GenVM project. Use after making code changes to compile Rust binaries.
---

To build the GenVM project:

**Build all Rust binaries:**
```bash
nix develop .#full --command bash .claude/skills/build/run-ninja.sh -C build all/bin
```

This runs ninja silently and only shows output on failure (to save tokens).

**Available ninja targets:**

| Target | Description |
|--------|-------------|
| `all` | Build everything |
| `all/bin` | Build all Rust binaries |
| `all/data` | Build data about runners using Nix |
| `codegen` | Run code generation |

**Output locations:**
- `out/bin/genvm-modules` - modules binary
- `out/executor/vTEST/bin/genvm` - executor binary

## Runners

Runners can only be built on x86_64 using the `all` target. On other platforms, download them instead.

**Download runners:**
```bash
nix develop .#full --command python3 build/out/bin/post-install.py --create-venv false --default-step false --runners-download true --error-on-missing-executor false
```

### Runner Development Workflow

To develop/modify a runner (e.g., cloudpickle):

1. **Enable dev mode:**
   Set `runners/support/current/dev-mode.nix` to `true`

2. **Set hash to "test":**
   In `runners/support/current/hashes.nix`, set the runner's hash to `"test"`

3. **Make your modifications and run tests**
   With dev-mode enabled and hash set to "test", you can build and run tests.

4. **Disable dev mode:**
   Set `runners/support/current/dev-mode.nix` back to `false`. The build will now tell you to set hashes to `null`.

5. **Set hashes to null and build:**
   Set the runner's hash (and dependent runners' hashes) to `null`, then build:
   ```bash
   nix develop .#full --command ninja -C build all
   ```
   The build will fail with a hash mismatch showing the new hash.

6. **Update hashes:**
   Copy the new hash from the error message back into `hashes.nix`. Repeat for dependent runners.

7. **Rebuild to verify:**
   Run the build again to confirm all hashes are correct.
