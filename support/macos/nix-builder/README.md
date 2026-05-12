# nix-builder

A Linux Nix remote builder reachable over SSH. Intended as a backing builder
for a macOS host that wants to build Linux derivations.

The container reads and writes a single volume mounted at
`/var/lib/nix-builder`, which holds the server's host key and the authorized
public key of the connecting host.

SSH login is `root` (single-user Nix). The connecting client only ever sees
that the Nix store is accessible — it does not need shell access beyond
`nix-store --serve` / `nix-daemon --stdio`.

## Prerequisites

- macOS host on Apple Silicon (Apple-Silicon Mac mini / MacBook).
- Rosetta installed system-wide:

  ```sh
  softwareupdate --install-rosetta --agree-to-license
  ```

- [Docker Desktop](https://www.docker.com/products/docker-desktop/) (4.25+),
  with **Settings → General → "Use Rosetta for x86_64/amd64 emulation on Apple
  Silicon"** enabled. Foreign-arch (`x86_64-linux`) builds are dispatched by
  the kernel's `binfmt_misc` to Rosetta; without it it may crash
- Nix on the host (the connecting client). Tested with Nix `2.34.7`; older
  `2.2?.*` releases mishandle the `ssh-ng://host:port` form.

## Build

```sh
docker build -t nix-builder .
# potentially you need to add
# --platform linux/arm64
```

## First run (bootstrap)

Drop the public key of the host that should connect (typically the macOS
machine's `~/.ssh/id_ed25519.pub`) into the volume as `host.pub`:

```sh
mkdir -p ./ssh
cp ~/.ssh/id_ed25519.pub ./ssh/host.pub
```

## Run

```sh
docker run -d \
    --name nix-builder --rm \
    --privileged \
    -p 2222:22 \
    -v "$PWD/ssh:/var/lib/nix-builder" \
    nix-builder
```

Privileged is needed for sandboxing

On startup the entrypoint:

1. Generates the server host key (`id_ed25519`) into the volume if absent.
2. Copies `host.pub` to `authorized_keys`.
3. Execs `sshd`.

Connect as `root@<host> -p 2222`.

Update `~/.ssh/known_hosts` with `ssh/id_ed25519.pub`:

```
[localhost]:2222 ssh-ed25519 <key>
```

> **Important:** you need to do it for root as well
> you can ensure it via `sudo ssh -i /Users/rentamac/.ssh/id_ed25519 -p 2222 root@localhost true`

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

# Appendix

For users unfammiliar with nix your `/etc/nix/nix.conf ` should look something like this

```
sandbox = true
experimental-features = nix-command flakes
sandbox-fallback = false
trusted-users = root YOUR_USER_NAME
builders-use-substitutes = true
```
