import pytest

from genlayer.storage import TreeMap, inmem_allocate


def new_map():
	return inmem_allocate(TreeMap[str, str])


def same_iter(li, ri):
	for l, r in zip(li, ri, strict=True):
		assert l == r


def test_construct():
	r = {str(i): str(i) + str(i) for i in range(10)}
	m = new_map()
	m.update(r)

	same_iter(m.items(), r.items())


@pytest.mark.parametrize('key', ['1', '-1', '2', '10'])
def test_contains(key: str):
	r = {str(i): str(i) + str(i) for i in range(10)}
	m = new_map()
	m.update(r)

	assert (key in m) == (key in r)


@pytest.mark.parametrize(
	'key,dflt', [(key, dflt) for key in ['1', '-1', '2', '10'] for dflt in [None, 'dflt']]
)
def test_get_dflt(key: str, dflt):
	r = {str(i): str(i) + str(i) for i in range(10)}
	m = new_map()
	m.update(r)

	assert m.get(key, dflt) == r.get(key, dflt)


@pytest.mark.parametrize('key', ['1', '-1', '2', '10'])
def test_get(key: str):
	r = {str(i): str(i) + str(i) for i in range(10)}
	m = new_map()
	m.update(r)

	assert m.get(key) == r.get(key)


@pytest.mark.parametrize('key', ['1', '-1', '2', '10', '1000'])
def test_set(key: str):
	r = {str(i): str(i) + str(i) for i in range(10)}
	m = new_map()
	m.update(r)

	m[key] = 'test'
	r[key] = 'test'

	same_iter(sorted(m.items()), sorted(r.items()))


@pytest.mark.parametrize('key', ['1', '2', '9'])
def test_del(key: str):
	r = {str(i): str(i) + str(i) for i in range(10)}
	m = new_map()
	m.update(r)

	del m[key]
	del r[key]

	same_iter(sorted(m.items()), sorted(r.items()))


def test_repr():
	v = {
		'b': 'русские буквы',
		'a': 'c',
	}

	m = new_map()
	m.update(v)

	assert repr(m) == "{'a':'c','b':'русские буквы'}"


def test_repr_empty():
	m = new_map()
	assert repr(m) == '{}'


def test_len():
	m = new_map()
	assert len(m) == 0
	m['a'] = 'x'
	assert len(m) == 1
	m['b'] = 'y'
	assert len(m) == 2
	del m['a']
	assert len(m) == 1


def test_clear():
	r = {str(i): str(i) for i in range(10)}
	m = new_map()
	m.update(r)
	assert len(m) == 10
	m.clear()
	assert len(m) == 0
	assert list(m) == []


def test_getitem():
	m = new_map()
	m['a'] = 'val_a'
	assert m['a'] == 'val_a'


def test_getitem_missing():
	m = new_map()
	with pytest.raises(KeyError):
		m['missing']


def test_del_missing():
	m = new_map()
	m['a'] = 'x'
	with pytest.raises(KeyError):
		del m['missing']


def test_iter():
	r = {str(i): str(i) for i in range(10)}
	m = new_map()
	m.update(r)
	assert sorted(m) == sorted(r)


def test_assign():
	m = new_map()
	m['pre'] = 'existing'
	r = {'a': '1', 'b': '2', 'c': '3'}
	m.assign(r)
	assert len(m) == len(r)
	same_iter(sorted(m.items()), sorted(r.items()))


def test_compute_if_absent_missing():
	m = new_map()
	result = m.compute_if_absent('k', lambda: 'new_val')
	assert result == 'new_val'
	assert m['k'] == 'new_val'


def test_compute_if_absent_existing():
	m = new_map()
	m['k'] = 'old_val'
	result = m.compute_if_absent('k', lambda: 'new_val')
	assert result == 'old_val'
	assert m['k'] == 'old_val'


def test_get_or_insert_default():
	m = new_map()
	m.get_or_insert_default('k')
	assert 'k' in m


def test_get_or_insert_default_existing():
	m = new_map()
	m['k'] = 'val'
	result = m.get_or_insert_default('k')
	assert result == 'val'


def test_items_contains():
	m = new_map()
	m['a'] = 'x'
	m['b'] = 'y'
	assert ('a', 'x') in m.items()
	assert ('a', 'z') not in m.items()
	assert ('c', 'x') not in m.items()


def test_items_len():
	m = new_map()
	m['a'] = 'x'
	m['b'] = 'y'
	assert len(m.items()) == 2


@pytest.mark.parametrize('key', ['0', '3', '5', '9'])
def test_setitem_overwrite(key: str):
	r = {str(i): str(i) + str(i) for i in range(10)}
	m = new_map()
	m.update(r)

	m[key] = 'overwritten'
	r[key] = 'overwritten'

	same_iter(sorted(m.items()), sorted(r.items()))


def test_del_all():
	r = {str(i): str(i) for i in range(10)}
	m = new_map()
	m.update(r)

	for k in list(r.keys()):
		del m[k]
		del r[k]
		assert len(m) == len(r)

	assert len(m) == 0


def test_del_reinsert():
	m = new_map()
	m['a'] = '1'
	del m['a']
	m['a'] = '2'
	assert m['a'] == '2'
	assert len(m) == 1
