{ pkgs
, lib
, rustShell
, ...
}@args:
let
	cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.lock);
	cargoPackageConfs = builtins.filter (p: builtins.hasAttr "source" p) cargoToml.package;
	cargoPackages = builtins.map (x: fetchCratesIo { inherit (x) version name; sha256 = x.checksum; }) cargoPackageConfs;
	fetchCratesIo = { name, version, sha256 }: pkgs.fetchurl {
		name = "${name}-${version}.crate";
		url = "https://crates.io/api/v1/crates/${name}/${version}/download";
		inherit sha256;
	};
in
rustShell.stdenv.mkDerivation {
	name = "genvm-cpython-ext";
	version = "0.0.1";

	outputHash = "sha256-ruKZP6DLrHeHuJIWTDAQQ/3CUvPnSrQ3PV0G3AamRr4=";
	outputHashMode = "recursive";

	buildInputs = [
	];
	nativeBuildInputs = [
		pkgs.zlib
		rustShell.rs
		pkgs.patchelf
		pkgs.glibc
		pkgs.perl
	];

	srcs = [
		./.
	] ++ cargoPackages;

	unpackPhase = ''
		mkdir -p ./cargo/registry/cache/index.crates.io-1949cf8c6b5b557f
		for file in $srcs
		do
			echo "$file"
			if ! (echo "$(stripHash "$file")" | grep -P '\.crate$')
			then
				cp -r "$file"/* .
			else
				cp "$file" "./cargo/registry/cache/index.crates.io-1949cf8c6b5b557f/$(stripHash "$file")"
			fi
		done

		mkdir -p ./cargo/registry/index/index.crates.io-1949cf8c6b5b557f/
		echo '{ "dl": "https://static.crates.io/crates", "api": "https://crates.io" }' > ./cargo/registry/index/index.crates.io-1949cf8c6b5b557f/config.json

		echo 'Signature: 8a477f597d28d172789f06886806bc55' > ./cargo/registry/CACHEDIR.TAG

		tar -C ./cargo/registry/index/index.crates.io-1949cf8c6b5b557f/ -xf ./registry.tar.xz

		perl -pe 's/^# //' -i Cargo.toml
	'';

	buildPhase = ''
		export CARGO_HOME="$(readlink -f ./cargo)"
		ls ./cargo/registry/cache/index.crates.io-1949cf8c6b5b557f/
		mkdir -p ./cargo
		cp -r "${rustShell.rs.outPath}/"* ./cargo
		chmod -R +w ./cargo
		for file in $(find ./cargo -type f -executable -and -not -name '*.sh')
		do
			echo "$file"
			patchelf --set-interpreter "${pkgs.glibc.outPath}/lib/ld-linux-x86-64.so.2" "$file" || true
		done
		cp "${pkgs.zlib.outPath}/lib/libz.so.1" ./cargo/lib/libz.so.1

		env PATH="$(readlink -f ./cargo/bin/):$PATH" \
			cargo --locked --offline build --target wasm32-wasip1 --profile release
	'';

	installPhase = ''
		mkdir -p "$out/lib"
		cp target/wasm32-wasip1/release/libgenvm_cpython_ext.a "$out/lib/libgenvm_cpython_ext.a"
		cp Setup.local "$out/"
	'';
}
