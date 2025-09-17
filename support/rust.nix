{ pkgs
, system ? "x86_64-linux"
, withLinters ? false
, withZig ? true
, ...
}@args:
let
	zig = import ./zig.nix args;

	systemAsRust = {
		x86_64-linux = "x86_64-unknown-linux-gnu";
		aarch64-linux = "aarch64-unknown-linux-gnu";
		aarch64-darwin = "aarch64-apple-darwin";
	}.${system};

	systemAsGenVM = {
		x86_64-linux = "amd64-linux";
		aarch64-linux = "arm64-linux";
		aarch64-darwin = "arm64-macos";
	}.${system};

	manifest-src = builtins.fetchurl {
		url = "https://static.rust-lang.org/dist/2025-03-18/channel-rust-stable.toml";
		sha256 = "02brsran14qag13vy082cmya52blj424grlpb902fbni1ilswz8y";
	};

	manifest = builtins.fromTOML (builtins.readFile manifest-src);

	simpleComponent = x: builtins.fetchurl {
			url = x.url;
			sha256 = x.hash;
		};

	components = [
		# core
		(simpleComponent manifest.pkg.cargo.target.${systemAsRust})
		(simpleComponent manifest.pkg.rustc.target.${systemAsRust})
		(simpleComponent manifest.pkg.rust-std.target.${systemAsRust})

		# cross compilation
		(simpleComponent manifest.pkg.rust-std.target.x86_64-unknown-linux-musl)
		(simpleComponent manifest.pkg.rust-std.target.aarch64-unknown-linux-musl)
		(simpleComponent manifest.pkg.rust-std.target.aarch64-apple-darwin)
	] ++ (if !withLinters then [] else [
		(simpleComponent manifest.pkg.clippy-preview.target.${systemAsRust})
		(simpleComponent manifest.pkg.rustfmt-preview.target.${systemAsRust})
		(simpleComponent manifest.pkg.llvm-tools-preview.target.${systemAsRust})
		(simpleComponent manifest.pkg.rust-src.target."*")
	]);

	rust-objcopy = pkgs.writeShellScript "rust-objcopy" ''
		exec zig objcopy "$@"
	'';
in pkgs.stdenvNoCC.mkDerivation rec {
	name = "genvm-rust";

	srcs = components;
	sourceRoot = ".";

	dontConfigure = true;
	dontBuild = true;

	nativeBuildInputs = [ pkgs.makeWrapper ];

	buildInputs = [
		pkgs.glibc
		pkgs.zlib
		pkgs.bash
		pkgs.gcc.cc.lib

		zig
	];

	dontAutoPatchelf = true;

	fixupPhase = ''
		find $out/bin -type f -executable | while read binary; do
			if file "$binary" | grep -q "ELF"
			then
				echo "Patching $binary"
				patchelf \
					--set-interpreter ${pkgs.glibc}/lib/ld-linux-x86-64.so.2 \
					--set-rpath "${pkgs.lib.makeLibraryPath buildInputs}:"'$ORIGIN/../lib' \
					"$binary"
			fi
		done

		find $out/lib -type f -maxdepth 1 | while read binary; do
			if file "$binary" | grep -q "ELF"
			then
				echo "Patching $binary"
				patchelf \
					--set-rpath "${pkgs.lib.makeLibraryPath buildInputs}:"'$ORIGIN/../lib' \
					"$binary"
			fi
		done

		runHook postInstall
	'';

	installPhase = ''
		mkdir -p $out
		for i in $(find . -type d -maxdepth 2 -mindepth 1) ;
		do
			cp -r "$i/." $out/.
		done

		ls -l "$out"

		cp ${rust-objcopy} $out/bin/rust-objcopy
	'' + (if withZig then ''
		wrapProgram $out/bin/cargo \
			--set CC_x86_64_unknown_linux_musl zig-cc-amd64-linux \
			--set CC_x86_64_unknown_linux_gnu zig-cc-amd64-linux-gnu \
			--set CC_aarch64_unknown_linux_musl zig-cc-arm64-linux \
			--set CC_aarch64_unknown_linux_gnu zig-cc-arm64-linux-gnu \
			--set CC_aarch64_apple_darwin zig-cc-arm64-macos \
			--set CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER zig-cc-amd64-linux \
			--set CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER zig-cc-arm64-linux \
			--set CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER zig-cc-arm64-macos \
			--prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath buildInputs}"

			#--set CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER zig-cc-arm64-linux-gnu \
			#--set CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER zig-cc-amd64-linux-gnu \
	'' else "");
}
