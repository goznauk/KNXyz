import json


from . import _knxyz


def encode(dpt: str, value) -> bytes:
    return _knxyz.encode_dpt_json(dpt, json.dumps(value))


def decode(dpt: str, data: bytes):
    return json.loads(_knxyz.decode_dpt_json(dpt, bytes(data)))


def bool(value: bool):
    return value
