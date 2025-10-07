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
		libc = build-libc { inherit pkgs; conf-target = "x86_64"; name-target = "amd64"; };
		liblua = build-lua {
			inherit pkgs zig;
			name-target = "amd64-linux";
		};
	};

	arm64-linux = {
		libc = build-libc { inherit pkgs; conf-target = "aarch64"; name-target = "arm64"; };
		liblua = build-lua {
			inherit pkgs zig;
			name-target = "arm64-linux";
		};
	};

	arm64-macos = {
		liblua = build-lua {
			inherit pkgs zig;
			name-target = "arm64-macos";
		};
		libiconv = import ./iconv.nix {
			inherit pkgs zig;
		};
	};
}
