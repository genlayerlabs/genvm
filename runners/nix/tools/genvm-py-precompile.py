import subprocess
import sys
from pathlib import Path

dir = Path(sys.argv[1])

for f in dir.glob('**/*.pyc'):
	f.unlink()

for f in dir.glob('**/__pycache__'):
	f.rmdir()

subprocess.run(
	[
		sys.executable,
		'-m',
		'compileall',
		'-d' '/py',
		'-o',
		'0',
		'-o',
		'2',
		'-f',
		'--invalidation-mode',
		'unchecked-hash',
		dir,
	],
	check=True,
	text=True,
)
