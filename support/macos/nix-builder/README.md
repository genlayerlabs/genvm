# nix-builder

A Linux Nix remote builder reachable over SSH. Intended as a backing builder
for a macOS host that wants to build Linux derivations (e.g. the genvm
`runners-all`, whose wasm/cpython toolchain only builds on `x86_64-linux`).

The container reads and writes a single volume mounted at
`/var/lib/nix-builder`, which holds the server's host key and the authorized
public key of the connecting host.

SSH login is `root` (single-user Nix). The connecting client only ever sees
that the Nix store is accessible — it does not need shell access beyond
`nix-store --serve` / `nix-daemon --stdio`.

## Prerequisites

- macOS host on Apple Silicon (Apple-Silicon Mac mini / MacBook).
- Nix on the host (the connecting client). Tested with Nix `2.34.7`; older
  `2.2?.*` releases mishandle the `ssh-ng://host:port` form.
- A Linux container runtime that can run `x86_64-linux`. Two options below —
  **prefer colima + QEMU** (Option A) for building the runners; see the Rosetta
  caveat.

### ⚠️ Rosetta caveat

Docker Desktop's **"Use Rosetta for x86_64/amd64 emulation"** mangles
versioned-symbol resolution for some x86_64 binaries. In particular the
wasi-sdk `clang` dies with:

```
clang: symbol lookup error: …/wasi-sdk/bin/clang: undefined symbol: , version LLVM_22.1
```

(note the empty symbol name) when building the runners (`genvm-bz2` → cpython →
wasm). This happens whether the container is arm64 (per-binary translation) or
amd64 (whole-container Rosetta) — it's **Rosetta**, not the container arch. Use
QEMU instead: either **Option A** below, or in Docker Desktop turn the Rosetta
option off so it falls back to QEMU.

## Option A — colima + QEMU (recommended)

```sh
brew install qemu lima-additional-guestagents colima

colima start qemuamd64 --arch x86_64 --vm-type qemu --cpu 8 --memory 8
# during iteration `colima delete qemuamd64` may be needed if it fails to start

docker --context colima-qemuamd64 build -t nix-builder .

docker --context colima-qemuamd64 run -d \
    --name nix-builder --rm \
    --privileged \
    -p 2222:22 \
    -v "$PWD/ssh:/var/lib/nix-builder" --cpus 8 --memory 10g \
    nix-builder
```

The container runs as native `x86_64` inside the QEMU VM
(`builtins.currentSystem == x86_64-linux`), so x86_64 builds never touch
Rosetta.

> **Bind-mount / ssh dir:** colima only mounts `$HOME` into its VM by default
> (not `/tmp`). Run from a path under `$HOME`, or stage the `ssh/` dir under
> `$HOME` (or add a colima `--mount`); otherwise the `-v "$PWD/ssh:…"` mount is
> empty inside the VM and the container can't read `host.pub`.

## Option B — Docker Desktop

[Docker Desktop](https://www.docker.com/products/docker-desktop/) (4.25+).
For x86_64 builds either enable QEMU (turn the Rosetta option **off** — see the
caveat above) or accept that pure-Linux builds that don't exercise the wasi-sdk
clang will work under Rosetta.

```sh
docker build -t nix-builder .
# on Apple Silicon you may need: --platform linux/amd64  (or linux/arm64)

docker run -d \
    --name nix-builder --rm \
    --privileged \
    -p 2222:22 \
    -v "$PWD/ssh:/var/lib/nix-builder" \
    nix-builder
```

`--privileged` is needed for the nix sandbox.

## Resources

QEMU emulation is slow, and the runner builds (cpython / numpy / Pillow → wasm)
are RAM-hungry. Give the VM real resources or the build will crawl / OOM:

- colima VM: `--cpu 8 --memory 8` (reference values; `--cpu 6 --memory 12` also
  works). The 2 CPU / 2 GiB colima default is not enough.
- per-container caps on `docker run`: `--cpus 8 --memory 10g`.

## First run (bootstrap)

Drop the public key of the host that should connect (typically the macOS
machine's `~/.ssh/id_ed25519.pub`) into the volume as `host.pub`:

```sh
mkdir -p ./ssh
cp ~/.ssh/id_ed25519.pub ./ssh/host.pub
```

On startup the entrypoint:

1. Generates the server host key (`id_ed25519`) into the volume if absent.
2. Copies `host.pub` to `authorized_keys`.
3. Execs `sshd`.

Connect as `root@<host> -p 2222`.

Update `~/.ssh/known_hosts` with `ssh/id_ed25519.pub`:

```
[localhost]:2222 ssh-ed25519 <key>
```

> **Important:** you need to do it for root as well (the nix-daemon ssh's to the
> builder as root). Ensure it via:
> `sudo ssh -i ~/.ssh/id_ed25519 -p 2222 root@localhost true`

## Sanity check

```sh
ssh root@localhost -p 2222 -- 'echo 123'
# must echo 123

NIX_REMOTE="ssh-ng://root@localhost:2222" nix build --system x86_64-linux nixpkgs#hello
# must exit successfully

NIX_REMOTE="ssh-ng://root@localhost:2222" nix build --system aarch64-linux nixpkgs#hello
# must exit successfully
```

> **Note:** tested on Nix `2.34.7`. Older (`2.2?.*`) versions may output
> `ssh: Could not resolve hostname localhost:2222: Name or service not known`.

## Using

Wire this up as a Nix remote builder by adding an entry to `/etc/nix/machines`
referencing the SSH host and the key:

```
ssh-ng://root@localhost:2222 x86_64-linux /Users/you/.ssh/id_ed25519 4 1 kvm,big-parallel
```

Then enable remote builders in `/etc/nix/nix.conf`:

```
builders-use-substitutes = true
```

Now a host-side build of an `x86_64-linux` derivation (e.g.
`nix build '.#runners-all'`) is dispatched to the container automatically.

## Notes on the image config

The image disables nix's seccomp syscall filter:

- build-time via `ENV NIX_CONFIG="filter-syscalls = false"` in the `Dockerfile`,
- runtime via `filter-syscalls = false` in `setup_nix_conf.py`.

Under emulation nix can't load its seccomp BPF program
(`error: unable to load seccomp BPF program: Invalid argument`), so the filter
must be off. `setup_nix_conf.py` also sets `system` to the container's native
arch, `extra-platforms` to the other Linux arch, and `cores = 0` (use all cores
so the build isn't single-threaded).

# Appendix

For users unfamiliar with nix your `/etc/nix/nix.conf` should look something
like this:

```
sandbox = true
experimental-features = nix-command flakes
sandbox-fallback = false
trusted-users = root YOUR_USER_NAME
builders-use-substitutes = true
```
