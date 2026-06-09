---
name: commit-style
description: GenVM commit message conventions. Use when writing a commit message, squashing/merging a PR, or amending history. Covers the `type(scope): summary <emoji>` format, the five types, the standard scope set, the gitmoji suffix, and mistakes to avoid (typos, vague "fix CI N").
---

# Writing commit messages in GenVM

Format:

```
type(scope): short imperative summary <emoji…>
```

- **type** — one of five, lowercase (required).
- **(scope)** — from the standard set below. Optional, but **aim for ~80%** of
  commits to carry one; omit only for genuinely repo-wide changes.
- **summary** — lowercase, imperative or noun-phrase, no trailing period, ≲70 chars.
- **\<emoji…>** — **one to three** trailing [gitmoji](https://gitmoji.dev) glyphs
  marking the precise intent. Use the actual Unicode emoji, not the `:shortcode:`.
  Most commits want exactly one; reach for a second or third only when the change
  genuinely carries more than one intent (e.g. a security fix that is also a
  refactor → `🔒️♻️`). Order them most-important first. Never more than three.

Examples:

```
feat(calldata): add lazy decoding ✨
perf(calldata): optimize internally tagged enums ⚡
fix(executor): report entire fees subtree to host 🐛
fix(calldata): defer parsing of forwarded calldata 🔒️⚡
chore(ci): fix macos cache key 💚
chore(build): bump wasmtime ⬆️
docs(executor): add spec for ram consumption 📝
```

## Types

The **type** is the coarse group; the **emoji** carries the fine intent.

| Type    | Use for                                        | Default emoji |
|---------|------------------------------------------------|---------------|
| `feat`  | new caller-visible capability or API surface   | ✨            |
| `fix`   | broken behavior corrected                      | 🐛            |
| `perf`  | same behavior, faster or smaller               | ⚡            |
| `docs`  | documentation / spec only                      | 📝            |
| `chore` | everything internal (incl. refactors)          | see below     |

Picking between the blurry ones:
- New behavior a caller can observe → `feat`. Correcting wrong behavior → `fix`.
  Same behavior but cheaper → `perf`. Pure restructure → `chore` + ♻️.

## Emoji by intent

Pick the glyph that best names *what kind* of change it is — it can be finer than
the type. Common ones:

| Emoji | Meaning                  | Emoji | Meaning                |
|-------|--------------------------|-------|------------------------|
| ✨    | new feature              | ♻️    | refactor               |
| 🐛    | bug fix                  | 🔥    | remove code / files    |
| 🚑    | critical hotfix          | 🎨    | structure / format     |
| ⚡    | performance              | ✅    | tests                  |
| 📝    | docs                     | 💚    | fix CI                 |
| 🔧    | config                   | 🚀    | release / deploy       |
| ⬆️/⬇️ | bump / drop deps         | 🔒️    | security / privacy     |
| 🚚    | move / rename            | 🔇    | remove logs            |
| 🏗️    | architectural change     | 🔨    | dev / build scripts    |
| 🚧    | work in progress         |       |                        |

Full reference: https://gitmoji.dev

A lot of `fix`es in this repo are security-relevant — untrusted calldata from
other nodes, fee/gas underflows, page limits, sandbox boundaries. When a fix
hardens behavior against malicious or malformed input, mark it 🔒️ (alone, or
paired with 🐛/⚡ when it is also a bugfix or optimization). The lazy-calldata
work, for instance, is partly about not eagerly parsing attacker-controlled
bytes — that earns a 🔒️.

## Standard scopes

| Scope       | Covers                                                         |
|-------------|----------------------------------------------------------------|
| `executor`  | rust executor core — calldata, host, common, rt/supervisor, fees, storage |
| `wasm`      | wasm/wasmtime compilation, precompile, wasm features           |
| `wasi`      | the wasi syscall layer (`executor/src/wasi`)                   |
| `rs-sdk`    | the rust SDK (`executor/crates/sdk-rs`)                        |
| `py-sdk`    | the python stdlib / SDK (`runners/genlayer-py-std`)           |
| `modules`   | modules in general (interfaces, install, implementation)       |
| `manager`   | the module manager (`modules/implementation/src/manager`)      |
| `lua`       | lua host scripting and configs (`genvm-lua`)                  |
| `webdriver` | the webdriver module                                           |
| `ci`        | GitHub workflows / CI                                          |
| `build`     | build system, nix, release packaging, dependency bumps         |

The set is curated, not closed — if a change clearly belongs to a subsystem not
listed, a sensible lowercase scope is fine. Prefer an existing one when it fits
(a `calldata` change is `(executor)`).

## Body — keep it rare

Prefer a single line. The summary should carry the change on its own; if you're
reaching for a body to explain *what* changed, tighten the subject instead.

Add a body **only** when a reader genuinely cannot reconstruct the *why* from the
diff — a non-obvious tradeoff, a workaround for an external bug, or a subtle
invariant. When you do, write one or two sentences of motivation, not a recap of
the diff and not a bullet list of squashed sub-commits.

## Mistakes to avoid (all seen in this repo's history)

1. **Numbered firefighting** — `chore: fix CI`, `fix CI 2` … `fix CI 5`. Say
   *what* broke: `chore(ci): fix macos cache key 💚`. A numbered run means the
   messages describe nothing.
2. **`batch update (#NNN)`** with no theme. A merged PR still deserves a one-line
   summary of what it does.
3. **Typos** — history has `reeipt`, `absolete`, `exremely`, `auto-formater`.
   Spell-check the subject; it is permanent.
4. **`chore: fix tests`** with no cause. Add it: `chore(executor): fix tests after
   wasmtime rebase ✅`.

## Checklist

- [ ] Right type (feat / fix / perf / docs / chore)?
- [ ] Scope present (target ~80%) and from the standard set?
- [ ] Lowercase, no period, ≲70 chars?
- [ ] One to three trailing gitmoji glyphs (most-important first) matching the intent?
- [ ] Single line — body only if the *why* is truly unrecoverable from the diff?
- [ ] Spell-checked?
