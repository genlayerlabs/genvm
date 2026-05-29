{ pkgs
, deps
, zig
, lua-src
, ...
}:
let
	lib = pkgs.lib;

	build-lua = import ./liblua.nix;
	build-libc = import ./libc.nix;
in {
	amd64-linux = [
		(build-libc { inherit pkgs deps; conf-target = "x86_64"; name-target = "amd64"; })
		(build-lua {
			inherit pkgs zig lua-src;
			name-target = "amd64-linux";
		})
	];

	arm64-linux = [
		(build-libc { inherit pkgs deps; conf-target = "aarch64"; name-target = "arm64"; })
		(build-lua {
			inherit pkgs zig lua-src;
			name-target = "arm64-linux";
		})
	];

	arm64-macos = [
		(build-lua {
			inherit pkgs zig lua-src;
			name-target = "arm64-macos";
		})
		(import ./iconv.nix {
			inherit pkgs deps zig;
		})
	];
}
