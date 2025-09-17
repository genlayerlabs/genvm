{ pkgs
, system ? "x86_64-linux"
, ...
}:
let
	zig = {
		x86_64-linux = builtins.fetchTarball {
			url = "https://ziglang.org/download/0.15.1/zig-x86_64-linux-0.15.1.tar.xz";
			sha256 = "0yar24a1qjg503czwnkdipky1cfb08k0mm9c8gwv827d33df1070";
		};
		aarch64-linux = builtins.fetchTarball {
			url = "https://ziglang.org/download/0.15.1/zig-aarch64-linux-0.15.1.tar.xz";
			sha256 = "19vcv7a1scm4wpj5cgv3dmzajz37fdyx5x1inxfgxzsavbmvq3zy";
		};
		aarch64-macos = builtins.fetchTarball {
			url = "https://ziglang.org/download/0.15.1/zig-aarch64-macos-0.15.1.tar.xz";
			sha256 = "0gsvr1cl5xpqh8an97hw1zqdbsr3ymw7hvlrqykxi25kp1jc3jvf";
		};
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
