import argparse, re, operator
from pathlib import Path
from ya_test_runner import SharedContext


from .collection import Env
import typing


def add_args(parser: argparse.ArgumentParser) -> None:
	parser.add_argument(
		'--test-name',
		type=str,
		help='Only run tests matching this regex',
		default='.*',
		metavar='REGEX',
	)

	parser.add_argument(
		'--test-tags',
		type=str,
		help='Only run tests matching this tags, `(a|b)&!c`',
		default='true',
		metavar='EXPR',
	)

	parser.add_argument(
		'--continue-from',
		type=str,
		help='Only run tests listed in the specified continue file (from a previous failed run)',
		default=None,
		metavar='FILE',
	)


def _tokenize_expr(expr: str) -> list[str]:
	tokens = []
	idx = 0
	while idx < len(expr):
		if expr[idx].isspace():
			idx += 1
			continue
		if expr[idx] in '()&|^!':
			tokens.append(expr[idx])
			idx += 1
			continue
		word_end = idx
		while word_end < len(expr) and expr[word_end].isalpha():
			word_end += 1
		if word_end == idx:
			raise ValueError(f'Unexpected character at position {idx}: {expr[idx]}')
		tokens.append(expr[idx:word_end])
		idx = word_end
	return tokens


def _parse_tags_expr(
	toks: list[str], priority: int
) -> typing.Callable[[frozenset[str]], bool]:
	if priority == 0:
		if toks[-1] == '(':
			toks.pop()
			ret = _parse_tags_expr(toks, 2)
			if toks[-1] != ')':
				raise ValueError('Unmatched parenthesis in tags expression')
			toks.pop()
			return ret
		if toks[-1] == '!':
			toks.pop()
			inner = _parse_tags_expr(toks, 0)
			return lambda tags: not inner(tags)
		tag = toks.pop()
		if not tag[0].isalpha():
			raise ValueError(f'Unexpected token in tags expression: {tag}')
		if tag == 'true':
			return lambda tags: True
		if tag == 'false':
			return lambda tags: False
		return lambda tags: tag in tags
	p = {
		1: [('&', operator.and_)],
		2: [('|', operator.or_), ('^', operator.xor)],
	}
	lst = p[priority]
	left = _parse_tags_expr(toks, priority - 1)
	while len(toks) > 0:
		found = False
		for sym, op in lst:
			if toks[-1] == sym:
				toks.pop()
				right = _parse_tags_expr(toks, priority - 1)
				prev_left = left
				left = lambda tags, l=prev_left, r=right, o=op: o(l(tags), r(tags))
				found = True
				break
		if not found:
			break
	return left


def _load_continue_file(shared: SharedContext, filepath: str) -> set[str]:
	"""Load test names from a continue file."""
	path = Path(filepath)
	if not path.is_absolute():
		# Try relative to artifacts_dir/continue first
		continue_path = shared.artifacts_dir / 'continue' / filepath
		if continue_path.exists():
			path = continue_path
		else:
			# Then try relative to root_dir
			path = shared.root_dir / filepath

	if not path.exists():
		raise ValueError(f'Continue file not found: {filepath}')

	content = path.read_text()
	tests = {line.strip() for line in content.splitlines() if line.strip()}
	shared.logger.info('Loaded continue file', path=str(path), test_count=len(tests))
	return tests


def run(shared: SharedContext, collection_env: Env) -> Env:
	new_cases = []
	test_name_filter = collection_env.args.test_name
	test_name_regex = re.compile(test_name_filter)

	tags_expr_toks = _tokenize_expr(collection_env.args.test_tags)
	tags_expr_toks.reverse()
	tags_expr = _parse_tags_expr(tags_expr_toks, 2)

	# Load continue file if specified
	continue_tests: set[str] | None = None
	if collection_env.args.continue_from:
		continue_tests = _load_continue_file(shared, collection_env.args.continue_from)

	for case in collection_env.cases:
		# Apply name regex filter
		if not test_name_regex.search(case.description.name):
			continue

		# Apply tags filter
		if not tags_expr(case.description.tags):
			continue

		# Apply continue file filter
		if continue_tests is not None and case.description.name not in continue_tests:
			continue

		new_cases.append(case)

	new_cases.sort(key=lambda c: c.description.name)

	return Env(
		cases=new_cases,
		args=collection_env.args,
	)
