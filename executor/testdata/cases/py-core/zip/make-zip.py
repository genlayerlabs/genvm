import zipfile
import io
from pathlib import Path

DEFAULT_TIME = (1980, 1, 1, 0, 0, 0)

root = Path(__file__).parent

fake_zip = io.BytesIO()
with zipfile.ZipFile(fake_zip, mode='w', compression=zipfile.ZIP_STORED) as zip_file:

	def add_file(dst: str, src: Path):
		with open(src, 'rb') as f:
			contents = f.read()
		info = zipfile.ZipInfo(dst, date_time=DEFAULT_TIME)
		zip_file.writestr(info, contents)

	add_file('contract.py', root.joinpath('empty.py'))
	add_file('contract.pyc', root.joinpath('contract.pyc'))
	add_file('runner.json', root.joinpath('runner.json'))
fake_zip.flush()

zip_contents = fake_zip.getvalue()
with open(root.joinpath('contract.zip'), 'wb') as f:
	f.write(zip_contents)
