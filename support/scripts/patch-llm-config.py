#!/usr/bin/env python3

import argparse
from pathlib import Path

from ruamel.yaml import YAML

parser = argparse.ArgumentParser()
parser.add_argument('--tag', required=True, help='current tag')
parser.add_argument('file', help='file to process')

args = parser.parse_args()

yaml = YAML(typ='rt')
text = Path(args.file).read_text()
doc = yaml.load(text)

import re

ver_regex = re.compile(r'v(\d+)\.(\d+)\.(\d+)')

del doc['backends']['dev']

import io

res = io.StringIO()
yaml.dump(doc, res)

yaml_val = res.getvalue()
if not yaml_val.endswith('\n'):
	yaml_val += '\n'

Path(args.file).write_text(yaml_val)
