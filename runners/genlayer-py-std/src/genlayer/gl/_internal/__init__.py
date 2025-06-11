import typing
from ...py.types import Lazy
import os


class LazyApi[T, **R](typing.Protocol):
	def __call__(self, *args: R.args, **kwargs: R.kwargs) -> T:
		"""
		Immediately execute and get the result
		"""
		...

	def lazy(self, *args: R.args, **kwargs: R.kwargs) -> Lazy[T]:
		"""
		Wrap evaluation into ``Lazy`` and return it
		"""
		...


def _lazy_api[T, **R](fn: typing.Callable[R, Lazy[T]]) -> LazyApi[T, R]:
	def eager(*args: R.args, **kwargs: R.kwargs) -> T:
		return fn(*args, **kwargs).get()

	if os.getenv('GENERATING_DOCS', 'false') == 'true':
		annots: dict = dict(fn.__annotations__)
		annots['return'] = annots['return'].__args__[0]
		eager.__annotations__ = annots
		eager.__module__ = fn.__module__
		import inspect
		import textwrap

		eager.__signature__ = inspect.signature(fn)
		eager.__doc__ = (
			textwrap.dedent(fn.__doc__ or '')
			+ '\n\n.. note::\n\tsupports ``.lazy()`` version, which will return :py:class:`~genlayer.py.types.Lazy`'
		)
	eager.__name__ = fn.__name__
	eager.lazy = fn
	return eager
