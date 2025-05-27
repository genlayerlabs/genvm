__all__ = ('Event',)

from genlayer.py._internal.event import Event


def _emit(self: Event) -> None:
	from genlayer.std.advanced import emit_raw_event

	emit_raw_event(self.name, self.indexed, self._blob)


Event.emit = _emit
