{ pkgs
, zig
, ...
}:
let
	lib = pkgs.lib;

	build-lua = import ./liblua.nix;
	build-libc = import ./libc.nix;
in {
	amd64-linux = {
		libc = build-libc { inherit pkgs zig; cc = "zig-cc-amd64-linux"; conf-target = "x86_64"; name-target = "amd64"; };
		liblua = build-lua {
			inherit pkgs zig;
			name-target = "amd64-linux";
		};
	};

	arm64-linux = {
		libc = build-libc { inherit pkgs zig; conf-target = "aarch64"; zig-target = "aarch64-linux"; name-target = "arm64"; };
		liblua = build-lua {
			inherit pkgs zig;
			name-target = "arm64-linux";
		};
	};

	macos-arm64 = {
		liblua = build-lua {
			inherit pkgs zig;
			name-target = "macos-arm64";
		};
	};
}
