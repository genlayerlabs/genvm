import argparse, re, operator
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
) -> typing.Callable[[set[str]], bool]:
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


def run(shared: SharedContext, collection_env: Env) -> Env:
	new_cases = []
	test_name_filter = collection_env.args.test_name
	test_name_regex = re.compile(test_name_filter)

	tags_expr_toks = _tokenize_expr(collection_env.args.test_tags)
	tags_expr_toks.reverse()
	tags_expr = _parse_tags_expr(tags_expr_toks, 2)

	for case in collection_env.cases:
		if test_name_regex.match(case.description.name) and tags_expr(
			case.description.tags
		):
			new_cases.append(case)
	new_cases.sort(key=lambda c: c.description.name)

	return Env(
		cases=new_cases,
		args=collection_env.args,
	)
