{ pkgs
, deps
, zig
, ...
}:
let
	lib = pkgs.lib;

	lua-src = builtins.fetchGit {
		url = "https://github.com/lua/lua.git";
		rev = "75ea9ccbea7c4886f30da147fb67b693b2624c26";
		shallow = true;
	};

	build-lua = import ./liblua.nix;
	build-libc = import ./libc.nix;
	build-lsqlite3 = import ./lsqlite3.nix;
in {
	amd64-linux = let
		liblua = build-lua {
			inherit pkgs zig lua-src;
			name-target = "amd64-linux";
		};
	in [
		(build-libc { inherit pkgs deps; conf-target = "x86_64"; name-target = "amd64"; })
		liblua
		(build-lsqlite3 {
			inherit pkgs zig deps lua-src liblua;
			name-target = "amd64-linux";
		})
	];

	arm64-linux = let
		liblua = build-lua {
			inherit pkgs zig lua-src;
			name-target = "arm64-linux";
		};
	in [
		(build-libc { inherit pkgs deps; conf-target = "aarch64"; name-target = "arm64"; })
		liblua
		(build-lsqlite3 {
			inherit pkgs zig deps lua-src liblua;
			name-target = "arm64-linux";
		})
	];

	arm64-macos = let
		liblua = build-lua {
			inherit pkgs zig lua-src;
			name-target = "arm64-macos";
		};
	in [
		liblua
		(import ./iconv.nix {
			inherit pkgs deps zig;
		})
		(build-lsqlite3 {
			inherit pkgs zig deps lua-src liblua;
			name-target = "arm64-macos";
		})
	];
}
