{ repo ? "https://github.com/genlayerlabs/genvm.git"
, pkgs
, ...
}:
let
	revs = [
		"86b7cce46d9dee4ed1fb76e8107e60617b7622db" # v0.2.0
		"de381dbc862575b2a4f3d43a1b96ec14814af9fd" # v0.2.3
		"fed444c0d9537f41a6ccafeac7c7507a2cd8f69e" # v0.2.4
	];

	# These frozen v0.2.x runner trees hardcode the upstream zlib tarball URL in
	# runners/cpython/deps/zlib.nix with NO mirror. The current ("head") recipe
	# fetches the SAME genvm-zlib-src output (identical name + sha256, via
	# dependency-urls.json) but with a mirror list — so both produce the same
	# /nix/store output path through two different .drvs. nix realizes only one of
	# them, non-deterministically: pick the head .drv and the dead zlib.net URL
	# falls back to the GCS mirror and builds; pick this frozen .drv and there is
	# no mirror, so it 404s and the whole runners-all build fails (~50% of
	# from-source CI builds). zlib.net moved 1.3.1 under /fossils/, hence the 404.
	# We rewrite the dead URL to the genvm-artifacts GCS mirror — the same artifact
	# the head recipe already falls back to, which we control — in a copy of the
	# fetched tree before importing it, so BOTH .drvs are resilient regardless of
	# which one nix schedules. fetchzip is content-addressed by its sha256, so the
	# genvm-zlib-src output path (and every downstream runner output hash) is
	# unchanged; only the source URL changes. The grep guard fails loudly if a rev
	# no longer carries the expected dead URL, so this never silently no-ops.
	#
	# IMPORTANT: --preserve=mode + a content-only rewrite of zlib.nix. The frozen
	# trees ship executable build scripts (e.g. numpy's deps/stub-clang.py, 0755);
	# a blanket `cp --no-preserve=mode` strips those bits and the numpy configure
	# phase dies with exit 126 (Permission denied). So we keep every file's mode
	# and touch ONLY zlib.nix's contents — the rest of the tree stays byte- and
	# mode-identical, so downstream runner FOD outputs match their pinned hashes.
	patchOldSrc = rev: src:
		pkgs.runCommandLocal
			"genvm-runners-src-${builtins.substring 0 12 rev}-zlib-mirror"
			{ }
			''
				cp -r --preserve=mode ${src} "$out"
				zlibNix="$out/runners/cpython/deps/zlib.nix"
				if ! grep -q 'https://www.zlib.net/zlib-1.3.1.tar.gz' "$zlibNix"; then
					echo "generation-00 zlib patch: expected dead URL not found in $zlibNix (rev ${rev}); patch is stale" >&2
					exit 1
				fi
				chmod u+w "$zlibNix"
				tmp="$(mktemp)"
				sed 's|https://www.zlib.net/zlib-1.3.1.tar.gz|https://storage.googleapis.com/genvm-artifacts/zlib-1.3.1.tar.gz|g' \
					"$zlibNix" > "$tmp"
				cat "$tmp" > "$zlibNix"
				rm -f "$tmp"
			'';

	mapRev = rev:
		let
			src = builtins.fetchGit {
				url = repo;
				inherit rev;

				shallow = true;
				submodules = true;
			};
			patchedSrc = patchOldSrc rev src;
		in
			builtins.map (x: x // { inherit rev; }) (import "${patchedSrc}/runners")
		;
in
	# list[{id, hash, rev, derivation}]
	builtins.concatLists (builtins.map mapRev revs)
