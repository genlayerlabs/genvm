{ pkgs
, lib
, runnersLib
, ...
}@args:
let
	makeSingle = {
		obj,
		rel,
		runner_json ? { MapFile = { to = "/py/libs/"; file = "src/"; }; },
	}:
		let
			runner_json_file = pkgs.writeText "runner.json" (builtins.toJSON runner_json);
		in runnersLib.package {
			hash = obj.hash;
			id = obj.id;

			baseDerivation = pkgs.stdenvNoCC.mkDerivation {
				name = "${obj.id}-${runnersLib.hashToIDHash obj.hash}-src";
				src = lib.sources.cleanSourceWith {
					src = ./${rel};
					filter = fName: type: lib.sources.cleanSourceFilter fName type && !(type == "directory" && fName == "__pycache__");
					name = "${obj.id}-${runnersLib.hashToIDHash obj.hash}-base-src";
				};

				phases = ["unpackPhase" "installPhase"];
				installPhase = ''
					mkdir -p "$out/src/${rel}"
					cp -r "." "$out/src/${rel}/."
					cp "${runner_json_file}" "$out/runner.json"
				'';
			};
		};
	pyLibs = runnersLib.hashes.pyLibs;
in [
	(makeSingle { obj = pyLibs.cloudpickle; rel = "cloudpickle"; })
	(makeSingle { obj = pyLibs.protobuf; rel = "google/protobuf"; })
	(makeSingle { obj = pyLibs.word_piece_tokenizer; rel = "word_piece_tokenizer"; })
]
