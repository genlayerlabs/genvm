{ pkgs
, lib
, runnersLib
, ...
}@args:
let
	middle_runner_seq = [
		{ SetArgs = ["py" "-u" "-c" "import contract; import genlayer.std._internal.runner"]; }
		{ Depends = "${runnersLib.hashes.pyLibs.cloudpickle.uid}"; }
		{ Depends = "${runnersLib.hashes.pyLibs."genlayer-std".uid}"; }
		{ Depends = "${runnersLib.hashes.cpython.uid}"; }
	];
in
	(runnersLib.toListExcluded runnersLib.hashes.pyLibs.genlayer-std (runnersLib.packageWithRunnerJSON {
		inherit (runnersLib.hashes.pyLibs.genlayer-std) id hash;

		expr = {
			MapFile = { file = "genlayer/"; to = "/py/libs/genlayer/"; };
		};

		baseDerivation = lib.sources.cleanSourceWith {
			src = ./src;
			filter = fName: type: lib.sources.cleanSourceFilter fName type && !(type == "directory" && fName == "__pycache__");
			name = "genlayer-std-base-src";
		};
	})) ++

	(runnersLib.toListExcluded runnersLib.hashes.pyLibs.genlayer-embeddings (runnersLib.packageWithRunnerJSON {
		inherit (runnersLib.hashes.pyLibs.genlayer-embeddings) id hash;

		expr = {
			Seq = [
				{ Depends = runnersLib.hashes.pyLibs.word_piece_tokenizer.uid; }
				{ Depends = runnersLib.hashes.pyLibs.protobuf.uid; }
				{ MapFile = { file = "genlayer_embeddings/"; to = "/py/libs/genlayer_embeddings/"; }; }
				{ MapFile = { file = "onnx/"; to = "/py/libs/onnx/"; }; }
			];
		};

		baseDerivation = lib.sources.cleanSourceWith {
			src = ./src-emb;
			filter = fName: type: lib.sources.cleanSourceFilter fName type && !(type == "directory" && fName == "__pycache__");
			name = "genlayer-std-base-src";
		};
	})) ++

	(runnersLib.toListExcluded runnersLib.hashes.wrappers.py-genlayer (runnersLib.packageGlue {
		inherit (runnersLib.hashes.wrappers.py-genlayer) id hash;

		expr = {
			Seq = [
				{ With = { runner = "<contract>"; action = { MapFile = { file = "file"; to = "/contract.py"; }; }; }; }
			] ++ middle_runner_seq;
		};
	})) ++

	(runnersLib.toListExcluded runnersLib.hashes.wrappers.py-genlayer-multi (runnersLib.packageGlue {
		inherit (runnersLib.hashes.wrappers.py-genlayer-multi) id hash;

		expr = {
			Seq = [
				{ With = { runner = "<contract>"; action = { MapFile = { file = "contract/"; to = "/contract/"; }; }; }; }
			] ++ middle_runner_seq;
		};
	})) ++

	[]
