{ pkgs
}:
let
	zig = builtins.fetchTarball {
		url = "https://ziglang.org/builds/zig-x86_64-linux-0.15.0-dev.1380+e98aeeb73.tar.xz";
		sha256 = "1s04ysvqn543dm3yya4xanipq1hfq6mhl6jn6pkb0gzkivl20iax";
	};

	make-cc-wrapper = trg: pkgs.writeShellScript "zig-cc-${trg}" ''
		export ZIG_GLOBAL_CACHE_DIR=/build/.zig-cache
		export ZIG_LOCAL_CACHE_DIR=/build/.zig-cache-local
		args=()
		for arg in "$@"; do
			if [[ "$arg" != --target=* ]]; then
				args+=("$arg")
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
		cp ${make-cc-wrapper "aarch64-macos"} "$out/bin/zig-cc-arm64-macos"
	'';
}
