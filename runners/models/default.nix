{ runnersLib
, ...
}:
[
	(runnersLib.packageWithRunnerJSON {
		inherit (runnersLib.hashes.models.all-MiniLM-L6-v2) id hash;

		baseDerivation = ./all-MiniLM-L6-v2;

		expr = {
			Seq = [
				{ MapFile = { file = "model.onnx"; to = "/models/all-MiniLM-L6-v2/model.onnx"; }; }
				{ MapFile = { file = "model.json"; to = "/models/all-MiniLM-L6-v2/model.json"; }; }
				{
					AddEnv = {
						name = "GENLAYER_EMBEDDINGS_MODELS";
						val = "/models/all-MiniLM-L6-v2:\${GENLAYER_EMBEDDINGS_MODELS}";
					};
				}
			];
		};
	})
]
