import pytest

from genlayer.py.storage._internal.generate import storage

import datetime


@storage
class Store:
	dt: datetime.datetime


@pytest.mark.parametrize(
	'expr',
	[
		datetime.datetime.now(),
		datetime.datetime.now().astimezone(datetime.timezone.utc),
		datetime.datetime.now().astimezone(datetime.timezone(datetime.timedelta(hours=4))),
		datetime.datetime.now().astimezone(datetime.timezone(datetime.timedelta(hours=2))),
		datetime.datetime.now().astimezone(datetime.timezone(datetime.timedelta(hours=-4))),
		datetime.datetime.now().astimezone(
			datetime.timezone(datetime.timedelta(hours=-11))
		),
		datetime.datetime.now().astimezone(datetime.timezone(datetime.timedelta(hours=11))),
		datetime.datetime.fromisoformat('2024-11-26T06:42:42.424242Z'),
	],
)
def test_dt(expr: datetime.datetime):
	st = Store()
	st.dt = expr
	assert expr == st.dt


from genlayer.py.storage import TreeMap


@storage
class Pr:
	x: TreeMap[str, str]


a = Pr()
a.x.update({'x': 'y'})
