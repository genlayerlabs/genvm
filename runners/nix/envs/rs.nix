{ pkgs
, lib
, ...
}@args:
let
	# using nightly because we require trim-paths
	configuration = builtins.fromTOML (builtins.readFile (builtins.fetchurl {
		url = "https://static.rust-lang.org/dist/2025-01-09/channel-rust-nightly.toml";
		sha256 = "sha256:18jzllfbki05pjgxkcx5s3z4s7rl43vsf6bv7vy0864saagaj3k4";
	}));
	libgcc = pkgs.stdenv.cc.cc.libgcc;
	rs = pkgs.stdenv.mkDerivation {
		pname = "genvm-det-rust-builder";
		version = "0.0.1";

		srcs = [
			(pkgs.fetchzip {
				url = configuration.pkg.cargo.target.x86_64-unknown-linux-gnu.url;
				hash = "sha256-gzztX+ShWGl5qn5tmWzsY8618m0sI7eoFErHk8EEhAo=";
				name = "genvm-det-rust-cargo";
			})
			(pkgs.fetchzip {
				url = configuration.pkg.rustc.target.x86_64-unknown-linux-gnu.url;
				hash = "sha256-oZUlKMft8zLfHq9O2VtuAReyyD2QcCxu2JYbS15QwR0=";
				name = "genvm-det-rust-rustc";
			})
			(pkgs.fetchzip {
				url = configuration.pkg.reproducible-artifacts.target.x86_64-unknown-linux-gnu.url;
				hash = "sha256-fXOXfQe5ecOyBy1lH3uiQ4f2QtgYpj6F82U4q+0PfxE=";
				name = "genvm-det-rust-reproducible-artifacts";
			})
			(pkgs.fetchzip {
				url = configuration.pkg.rust-std.target.wasm32-wasip1.url;
				hash = "sha256-Ado7A3yg/ZPmhdFGSUd7yP+sOEQT6AWe8UG4leP9Jsc=";
				name = "genvm-det-rust-std-target-wasm32-wasip1";
			})
			(pkgs.fetchzip {
				url = configuration.pkg.rust-std.target.x86_64-unknown-linux-gnu.url;
				hash = "sha256-dI6fRPSqGYqccs3CujQLetatrw8HrzQoyvA5+IVeOKU=";
				name = "genvm-det-rust-std-target-amd64";
			})
		];

		outputHash = "sha256-LcPO+QHtv0Y/IKobI6Y1cuzbi6YKUf0aOIn7PJrYcbw="; #lib.fakeHash;
		outputHashMode = "recursive";

		buildInputs = [
			pkgs.zlib
			libgcc
		];

		nativeBuildInputs = with pkgs; [
			perl
		];

		unpackPhase = ''
			for i in $srcs
			do
				cp -r "$i" "./$(stripHash "$i")"
			done
			chmod -R +w .
		'';

		dontConfigure = true;

		dontBuild = true;

		installPhase = ''
			for i in genvm-det-rust-*
			do
				echo "$i"
				bash "./$i/install.sh" --disable-ldconfig --prefix="$out"
			done

			find "$out" -name '*.log' -delete
			rm -rf "$out/share/man" "$out/bin/rust-gdb"* "$out/bin/rust-lldb"* "$out/lib/rustlib/uninstall.sh" || true
			find "$out" -type f -name 'manifest-*' -print0 | xargs -0 perl -pe 's/$ENV{out}/\/rust-toolchain/g' -i
		'';

		dontFixup = true;
	};
in {
	inherit rs;

	stdenv = pkgs.stdenv;
}
