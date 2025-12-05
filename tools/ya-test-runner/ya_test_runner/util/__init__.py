from pathlib import Path
import shutil

from . import environ


def discover_executable(name: str) -> Path:
	exec_path = shutil.which(name)
	if exec_path is None:
		raise FileNotFoundError(f'Cannot find executable: {name}')
	return Path(exec_path)
