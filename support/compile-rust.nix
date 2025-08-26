{ pkgs
, zig
, ...
}@args:
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

	rust-pkg = import ./rust.nix args;
in

stdenv.mkDerivation (
	(removeAttrs args [
		"depsExtraArgs"
		"cargoUpdateHook"
		"cargoDeps"
		"cargoLock"
	])
	// lib.optionalAttrs (stdenv.isDarwin && buildType == "debug") {
		RUSTFLAGS =
			"-C split-debuginfo=packed "
			+ (args.RUSTFLAGS or "");
	}
	// {
		cargoDeps = importCargoLock cargoLock;
		inherit buildAndTestSubdir;

		RUSTFLAGS =
			"-C target-feature=-crt-static -l dylib=c -L /build/libs -C link-arg=-dynamic "
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

		dontFixup = true;
	}
)
