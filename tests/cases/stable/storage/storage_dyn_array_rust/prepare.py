import subprocess
import json
from pathlib import Path

root = Path(__file__).parent
repo_root = root.parents[4]

build_info = json.loads(repo_root.joinpath('build', 'info.json').read_text())
target_dir = Path(build_info['rust_target_dir'])

subprocess.run(
	[
		'cargo',
		'build',
		'--example',
		'storage_dyn_array',
		'--target',
		'wasm32-wasip1',
		'--release',
		'--target-dir',
		str(target_dir),
	],
	cwd=repo_root / 'executor' / 'crates' / 'sdk-rs',
	check=True,
)

src = target_dir / 'wasm32-wasip1' / 'release' / 'examples' / 'storage_dyn_array.wasm'
dst = root / 'storage_dyn_array.wasm'
wat = root / 'storage_dyn_array.wat'

# Round-trip through wabt to normalize to MVP format
subprocess.run(['wasm2wat', str(src), '-o', str(wat)], check=True)
subprocess.run(['wat2wasm', str(wat), '-o', str(dst)], check=True)
wat.unlink()
