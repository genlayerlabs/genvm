__all__ = ('Event',)

from genlayer.py._internal.event import Event
from genlayer.py.keccak import Keccak256
import genlayer.py.calldata as calldata


def _emit(self: Event) -> None:
	from genlayer.gl.advanced import emit_raw_event

	topics = [Keccak256(self.signature.encode('utf-8')).digest()]
	for i in self.indexed:
		d = self._blob[i]
		as_cd = calldata.encode(d)
		if len(as_cd) > 32:
			as_cd = Keccak256(as_cd).digest()
		topics.append(as_cd)

	emit_raw_event(topics, self._blob)


Event.emit = _emit
