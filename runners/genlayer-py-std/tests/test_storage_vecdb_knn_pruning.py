import numpy as np

from genlayer_embeddings import VecDB, EuclideanDistance
from genlayer.storage._internal.generate import generate_storage
import typing


@generate_storage
class DB:
	x: VecDB[np.int32, typing.Literal[5], str, EuclideanDistance]


def _vec(a: int) -> np.ndarray:
	return np.array([a, 0, 0, 0, 0], dtype=np.int32)


def _brute_nearest(coords: list[int], q: int) -> int:
	return min(coords, key=lambda c: (c - q) ** 2)


def test_knn_does_not_prune_true_nearest():
	"""Regression test for KNN pruning skipping the true nearest neighbor.

	``knn`` prunes a child subtree when ``child_dist - _max_descendant_dist >
	best_kth_dist``. That bound is only valid for a metric, but the default
	``EuclideanDistanceSquared`` returns *squared* Euclidean distance, which
	does not satisfy the triangle inequality. With the points below the cover
	tree forms a root -> child -> grandchild chain along a single axis; the
	query sits near the grandchild while the child is farther away, so the
	child subtree gets pruned after the root fills the best heap even though
	the grandchild is the actual nearest neighbor.
	"""

	# Inserted in this order they build a root(106) -> ... chain; querying near
	# 4 must return 106 (squared dist 10404), not 149 (squared dist 21025).
	coords = [106, 192, 300, 149]

	db = DB()
	for c in coords:
		db.x.insert(_vec(c), str(c))

	q = 4
	expected = str(_brute_nearest(coords, q))  # '106'

	got = [e.value for e in db.x.knn(_vec(q), 1)]

	assert got == [expected], (
		f'knn returned {got}, expected nearest {expected!r}; '
		f'squared distances were {({c: (c - q) ** 2 for c in coords})}'
	)
