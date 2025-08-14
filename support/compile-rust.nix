{ pkgs
, zig
}:
let
	lib = pkgs.lib;
	importCargoLock = pkgs.rustPlatform.importCargoLock;
	fetchCargoTarball = pkgs.rustPlatform.fetchCargoTarball;
	fetchCargoVendor = pkgs.rustPlatform.fetchCargoVendor;
	stdenv = pkgs.stdenv;
	callPackage = pkgs.callPackage;
	cargoBuildHook = pkgs.rustPlatform.cargoBuildHook;
	cargoInstallHook = pkgs.rustPlatform.cargoInstallHook;
	cargoSetupHook = pkgs.rustPlatform.cargoSetupHook;
	cargo = pkgs.cargo;
	cargo-auditable = pkgs.cargo-auditable;
	buildPackages = pkgs.buildPackages;
	libiconv = pkgs.libiconv;
	windows = pkgs.windows;
in {
	name ? "${args.pname}-${args.version}",

	# Name for the vendored dependencies tarball
	cargoDepsName ? name,

	src ? null,
	srcs ? null,
	preUnpack ? null,
	unpackPhase ? null,
	postUnpack ? null,
	cargoPatches ? [ ],
	patches ? [ ],
	sourceRoot ? null,
	logLevel ? "",
	buildInputs ? [ ],
	nativeBuildInputs ? [ ],
	cargoUpdateHook ? "",
	cargoDepsHook ? "",
	buildType ? "release",
	meta ? { },
	cargoLock,
	buildNoDefaultFeatures ? false,
	buildFeatures ? [ ],
	auditable ? !cargo-auditable.meta.broken,

	extraLibs ? [ ],

	depsExtraArgs ? { },

	# Toggles whether a custom sysroot is created when the target is a .json file.
	__internal_dontAddSysroot ? false,

	# Needed to `pushd`/`popd` into a subdir of a tarball if this subdir
	# contains a Cargo.toml, but isn't part of a workspace (which is e.g. the
	# case for `rustfmt`/etc from the `rust-sources).
	# Otherwise, everything from the tarball would've been built/tested.
	buildAndTestSubdir ? null,

	target,
	...
}@args:
let
	targetAsRust = {
		amd64-linux = "x86_64-unknown-linux-musl";
		arm64-linux = "aarch64-unknown-linux-musl";
		arm64-macos = "aarch64-apple-darwin";
	}.${target};

	targetIsJSON = lib.hasSuffix ".json" target;
	useSysroot = targetIsJSON && !__internal_dontAddSysroot;

	sysroot = callPackage ./sysroot { } {
		inherit target;
		shortTarget = target;
		RUSTFLAGS = args.RUSTFLAGS or "";
		originalCargoToml = src + /Cargo.toml; # profile info is later extracted
	};

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
		(simpleComponent manifest.pkg.cargo.target.x86_64-unknown-linux-gnu)
		(simpleComponent manifest.pkg.rustc.target.x86_64-unknown-linux-gnu)
		(simpleComponent manifest.pkg.rust-std.target.x86_64-unknown-linux-gnu)

		(simpleComponent manifest.pkg.rust-std.target.x86_64-unknown-linux-musl)
		(simpleComponent manifest.pkg.rust-std.target.aarch64-unknown-linux-musl)
		(simpleComponent manifest.pkg.rust-std.target.aarch64-apple-darwin)
	];

	rust-pkg = pkgs.stdenvNoCC.mkDerivation rec {
		name = "genvm-rust";

		srcs = components;
		sourceRoot = ".";

		dontConfigure = true;
		dontBuild = true;

		nativeBuildInputs = [];

		buildInputs = [
			pkgs.glibc
			pkgs.zlib
			pkgs.bash
			pkgs.gcc.cc.lib
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
		'';

		installPhase = ''
			mkdir -p $out
			for i in $(find . -type d -maxdepth 2 -mindepth 1) ;
			do
				cp -r "$i/." $out/.
			done

			ls -l "$out"
		'';
	};
in

stdenv.mkDerivation (
	(removeAttrs args [
		"depsExtraArgs"
		"cargoUpdateHook"
		"cargoDeps"
		"cargoLock"
	])
	// lib.optionalAttrs useSysroot {
		RUSTFLAGS = "--sysroot ${sysroot} " + (args.RUSTFLAGS or "");
	}
	// lib.optionalAttrs (stdenv.isDarwin && buildType == "debug") {
		RUSTFLAGS =
			"-C split-debuginfo=packed "
			+ lib.optionalString useSysroot "--sysroot ${sysroot} "
			+ (args.RUSTFLAGS or "");
	}
	// {
		cargoDeps = importCargoLock cargoLock;
		inherit buildAndTestSubdir;

		RUSTFLAGS =
			"-C target-feature=-crt-static -l dylib=c -L /build/libs -C link-arg=-dynamic "
			+ lib.optionalString useSysroot "--sysroot ${sysroot} "
			+ (args.RUSTFLAGS or "");

		hardeningDisable = ["all"];

		cargoBuildType = buildType;

		cargoBuildNoDefaultFeatures = buildNoDefaultFeatures;

		cargoBuildFeatures = buildFeatures;

		nativeBuildInputs =
			nativeBuildInputs
			++ [
				cargoSetupHook
				rust-pkg
				pkgs.strace
				zig
			];

		buildInputs =
			buildInputs
			++ lib.optionals stdenv.hostPlatform.isDarwin [ libiconv ];

		patches = cargoPatches ++ patches;

		PKG_CONFIG_ALLOW_CROSS = 1;

		postUnpack =
			''
				eval "$cargoDepsHook"

				mkdir -p /build/libs

				export RUST_LOG=${logLevel}
			''
			+ (args.postUnpack or "")
			+ "\n"
			+ builtins.concatStringsSep "\n" (
				builtins.map (x: "cp ${x}/lib/* /build/libs/") extraLibs
			);

		configurePhase =
			args.configurePhase or ''
				runHook preConfigure
				runHook postConfigure
			'';

		doCheck = false;

		strictDeps = true;

		meta = meta;

		buildPhase = ''
		runHook preBuild

		ls -l /build/libs/

		echo "RUSTFLAGS=$RUSTFLAGS"
		echo cargo build --target ${targetAsRust} -j $NIX_BUILD_CORES --offline --${buildType}
		env \
			RUSTFLAGS="$RUSTFLAGS" \
			CC=zig-cc-amd64-linux \
			CC_x86_64-unknown-linux-musl=zig-cc-amd64-linux \
			CC_aarch64-unknown-linux-musl=zig-cc-arm64-linux \
			CC_aarch64-apple-darwin=zig-cc-arm64-macos \
			CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=zig-cc-amd64-linux \
			CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=zig-cc-arm64-linux \
			CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=zig-cc-arm64-macos \
			cargo build --target ${targetAsRust} -j $NIX_BUILD_CORES --offline --${buildType}
		runHook postBuild

		bins=$(find target/${targetAsRust}/${buildType}/ \
				-maxdepth 1 \
				-type f \
				-executable -not -regex ".*\.\(so.[0-9.]+\|so\|a\|dylib\)" )
			echo "Found binary $bins"

		cp "$bins" target/__out

		patchelf --set-rpath '$ORIGIN/../lib:$ORIGIN/../../lib:' target/__out

		for i in $(patchelf --print-needed target/__out)
		do
			if [[ "$i" == /build/libs/* ]]
			then
				echo "Replacing $i with $(basename $i)"
				patchelf --replace-needed "$i" "$(basename $i)" target/__out
			fi
		done
		'';

		installPhase = ''
			cp "target/__out" "$out"
		'';

		dontFuxup = true;
	}
)
