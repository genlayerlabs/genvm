# NOTE: this file is not part of original onnx, it was modified, but contains snippets from there

import os
import typing
from typing import IO, Literal, Union

from .onnx_ml_pb2 import *
from .onnx_data_pb2 import *

_SupportedFormat = Union[
    Literal["protobuf", "textproto", "onnxtxt", "json"], str  # noqa: PYI051
]
# Default serialization format
_DEFAULT_FORMAT = "protobuf"

from onnx import serialization

def _get_file_path(f: IO[bytes] | str | os.PathLike | None) -> str | None:
    if isinstance(f, (str, os.PathLike)):
        return os.path.abspath(f)
    if hasattr(f, "name"):
        assert f is not None
        return os.path.abspath(f.name)
    return None


def _get_serializer(
    fmt: _SupportedFormat | None, f: str | os.PathLike | IO[bytes] | None = None
) -> serialization.ProtoSerializer:
    """Get the serializer for the given path and format from the serialization registry."""
    # Use fmt if it is specified
    if fmt is not None:
        return serialization.registry.get(fmt)

    if (file_path := _get_file_path(f)) is not None:
        _, ext = os.path.splitext(file_path)
        fmt = serialization.registry.get_format_from_file_extension(ext)

    # Failed to resolve format if fmt is None. Use protobuf as default
    fmt = fmt or _DEFAULT_FORMAT
    assert fmt is not None

    return serialization.registry.get(fmt)

def _load_bytes(f: IO[bytes] | str | os.PathLike) -> bytes:
    if hasattr(f, "read") and callable(typing.cast(IO[bytes], f).read):
        content = typing.cast(IO[bytes], f).read()
    else:
        f = typing.cast(Union[str, os.PathLike], f)
        with open(f, "rb") as readable:
            content = readable.read()
    return content

def load_model(
    f: IO[bytes] | str | os.PathLike,
    format: _SupportedFormat | None = None,  # noqa: A002
    load_external_data: bool = True,
) -> ModelProto:
    """Loads a serialized ModelProto into memory.

    Args:
        f: can be a file-like object (has "read" function) or a string/PathLike containing a file name
        format: The serialization format. When it is not specified, it is inferred
            from the file extension when ``f`` is a path. If not specified _and_
            ``f`` is not a path, 'protobuf' is used. The encoding is assumed to
            be "utf-8" when the format is a text format.
        load_external_data: Whether to load the external data.
            Set to True if the data is under the same directory of the model.
            If not, users need to call :func:`load_external_data_for_model`
            with directory to load external data from.

    Returns:
        Loaded in-memory ModelProto.
    """
    model = _get_serializer(format, f).deserialize_proto(_load_bytes(f), ModelProto())

    if load_external_data:
        model_filepath = _get_file_path(f)
        if model_filepath:
            base_dir = os.path.dirname(model_filepath)
            raise Exception(f"external data not supported, sorry =( {model} {base_dir}")
            #load_external_data_for_model(model, base_dir)

    return model
