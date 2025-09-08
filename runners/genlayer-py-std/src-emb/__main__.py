from genlayer_embeddings._nn import get_run_onnx

import argparse
import numpy as np
import onnx
import json
from pathlib import Path

arg_parser = argparse.ArgumentParser()
arg_parser.add_argument('--model-json', type=str, required=True)
arg_parser.add_argument('--output-py', type=str, required=True)
args = arg_parser.parse_args()

inputs = {
	'input_ids': np.int64,
	'attention_mask': np.int64,
	'token_type_ids': np.int64,
}

user_inputs = {}
for k in inputs.keys():
	user_inputs[k] = k

model_json = Path(args.model_json)
model_desc = json.loads(model_json.read_text())
model_path = model_json.parent.joinpath('model.onnx')

onnx_model = onnx.load_model(model_path, load_external_data=False)
rename_outputs = model_desc.get('rename-outputs', {})
builder, inp = get_run_onnx(
	onnx_model, user_inputs, rename_outputs, extra_builder_args={'compress': True}
)

builder._prelude.append(
	f'tokens_truncate = {repr(model_desc.get("tokens_truncate", None))}\n'
)

as_str = builder.finish_str(parameters=inp)

Path(args.output_py).write_text(as_str)
