{ pkgs
, deps
, system ? "x86_64-linux"
, ...
}:
let
	zig = {
		x86_64-linux = deps."zig-x86_64-linux";
		aarch64-linux = deps."zig-aarch64-linux";
		aarch64-macos = deps."zig-aarch64-macos";
	}.${system};

	make-cc-wrapper = trg: pkgs.writeShellScript "zig-cc-${trg}" ''
		if [ ! -d "$HOME" ]; then
			export ZIG_GLOBAL_CACHE_DIR=/build/.zig-cache
			export ZIG_LOCAL_CACHE_DIR=/build/.zig-cache-local
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
				[[ "$arg" != -F* ]]; then
				args+=("$arg")
			elif [[ "$arg" == -framework ]] || [[ "$arg" == -F ]]; then
				skip_next=true
			fi
		done

		exec "${zig}/zig" cc -fdebug-prefix-map=${toString zig}=/zig -target ${trg} "''${args[@]}"
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
