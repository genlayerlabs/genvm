class AlreadySerializedResult(bytes):
	def __new__(cls, *args, **kwargs):
		return bytes.__new__(cls, *args, **kwargs)
