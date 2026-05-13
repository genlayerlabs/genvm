{ pkgs
, deps
, system ? "x86_64-linux"
, ...
}:
let
	zig-system = {
		"x86_64-linux" = "x86_64-linux";
		"aarch64-linux" = "aarch64-linux";
		"aarch64-darwin" = "aarch64-macos";
	}.${system};
	zig = deps.${"zig-" + zig-system + "-0.15.1"};

	make-cc-wrapper = trg: pkgs.writeShellScript "zig-cc-${trg}" ''
		if [ ! -d "$HOME" ]; then
			export ZIG_GLOBAL_CACHE_DIR=$NIX_BUILD_TOP/.zig-cache
			export ZIG_LOCAL_CACHE_DIR=$NIX_BUILD_TOP/.zig-cache-local
		fi
		args=()
		for arg in "$@"; do
			if [[ "$skip_next" == true ]]; then
				skip_next=false
				continue
			fi
			# Filter Rust self-contained CRT objects — Zig provides its own.
			# Without this, both rcrt1.o (Rust) and crt1.o (Zig) define _start.
			if [[ "$arg" == */self-contained/rcrt1.o ]] || \
				[[ "$arg" == */self-contained/crti.o ]] || \
				[[ "$arg" == */self-contained/crtn.o ]] || \
				[[ "$arg" == */self-contained/crtbeginS.o ]] || \
				[[ "$arg" == */self-contained/crtendS.o ]]; then
				continue
			fi
			if [[ "$arg" != --target=* ]] && \
				[[ "$arg" != -framework ]] && \
				[[ "$arg" != CoreFoundation ]] && \
				[[ "$arg" != Foundation ]] && \
				[[ "$arg" != *CoreFoundation* ]] && \
				[[ "$arg" != *Foundation* ]] && \
				[[ "$arg" != -F ]] && \
				[[ "$arg" != -F* ]] && \
				[[ "$arg" != -Wl,-exported_symbols_list ]]; then
				args+=("$arg")
			elif [[ "$arg" == -framework ]] || [[ "$arg" == -F ]] || [[ "$arg" == -Wl,-exported_symbols_list ]]; then
				skip_next=true
			fi
		done

		# Append -L paths from NIX_LDFLAGS so libs from {native,}buildInputs are visible
		nix_ld_args=()
		read -r -a _nix_ld <<< "''${NIX_LDFLAGS:-} ''${NIX_LDFLAGS_FOR_BUILD:-}"
		take_next=false
		for ldarg in "''${_nix_ld[@]}"; do
			if [[ "$take_next" == true ]]; then
				nix_ld_args+=("-L" "$ldarg")
				take_next=false
			elif [[ "$ldarg" == "-L" ]]; then
				take_next=true
			elif [[ "$ldarg" == -L* ]]; then
				nix_ld_args+=("$ldarg")
			fi
		done

		exec "${zig}/zig" cc -fdebug-prefix-map=${toString zig}=/zig -target ${trg} "''${nix_ld_args[@]}" "''${args[@]}"
	'';
in
pkgs.stdenvNoCC.mkDerivation {
	name = "genvm-zig";

	src = zig;

	nativeBuildInputs = [ pkgs.coreutils ];

	doNotConfigure = true;

	doNotBuild = true;

	installPhase = ''
		mkdir -p "$out/bin"
		cp -r "./." "$out"
		cp ${make-cc-wrapper "x86_64-linux-musl"} "$out/bin/zig-cc-amd64-linux"
		cp ${make-cc-wrapper "aarch64-linux-musl"} "$out/bin/zig-cc-arm64-linux"
		cp ${make-cc-wrapper "x86_64-linux-gnu"} "$out/bin/zig-cc-amd64-linux-gnu"
		cp ${make-cc-wrapper "aarch64-linux-gnu"} "$out/bin/zig-cc-arm64-linux-gnu"
		cp ${make-cc-wrapper "aarch64-macos"} "$out/bin/zig-cc-arm64-macos"
	'';
}
