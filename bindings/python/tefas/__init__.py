import ctypes
import json
import os
from pathlib import Path


def _workspace_root() -> Path:
    for parent in Path(__file__).resolve().parents:
        if (parent / "Cargo.toml").exists() and (parent / "etl").exists():
            return parent
    raise RuntimeError("workspace root not found")


def _load_library() -> ctypes.CDLL:
    env_path = os.getenv("TEFAS_FFI_LIB")
    if env_path:
        return ctypes.CDLL(env_path)

    workspace_root = _workspace_root()
    candidates = [
        workspace_root / "target" / "release" / "libtefas_ffi.so",
        workspace_root / "target" / "debug" / "libtefas_ffi.so",
    ]
    for candidate in candidates:
        if candidate.exists():
            return ctypes.CDLL(str(candidate))

    raise RuntimeError(
        "libtefas_ffi.so not found. Build with: cargo build -p tefas-ffi --release"
    )


_lib = _load_library()

_lib.tefas_free_string.argtypes = [ctypes.c_char_p]
_lib.tefas_free_string.restype = None

_lib.tefas_parse_document_json.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p)]
_lib.tefas_parse_document_json.restype = ctypes.c_int

_lib.tefas_fetch_text_default.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_char_p), ctypes.POINTER(ctypes.c_char_p)]
_lib.tefas_fetch_text_default.restype = ctypes.c_int

_lib.tefas_query_operation_json_default.argtypes = [
    ctypes.c_char_p,
    ctypes.c_char_p,
    ctypes.POINTER(ctypes.c_char_p),
    ctypes.POINTER(ctypes.c_char_p),
]
_lib.tefas_query_operation_json_default.restype = ctypes.c_int


def _consume_ptr(ptr: ctypes.c_char_p) -> str:
    if not ptr:
        return ""
    raw = ctypes.string_at(ptr)
    text = raw.decode("utf-8")
    _lib.tefas_free_string(ptr)
    return text


def parse_document(html: str) -> dict:
    out = ctypes.c_char_p()
    err = ctypes.c_char_p()
    rc = _lib.tefas_parse_document_json(html.encode("utf-8"), ctypes.byref(out), ctypes.byref(err))
    if rc != 0:
        raise RuntimeError(_consume_ptr(err) or "tefas_parse_document_json failed")
    return json.loads(_consume_ptr(out))


def fetch_text(url: str) -> str:
    out = ctypes.c_char_p()
    err = ctypes.c_char_p()
    rc = _lib.tefas_fetch_text_default(url.encode("utf-8"), ctypes.byref(out), ctypes.byref(err))
    if rc != 0:
        raise RuntimeError(_consume_ptr(err) or "tefas_fetch_text_default failed")
    return _consume_ptr(out)


def query_operation(operation: str, payload: dict | None = None) -> dict:
    out = ctypes.c_char_p()
    err = ctypes.c_char_p()
    payload_raw = None if payload is None else json.dumps(payload)
    payload_arg = None if payload_raw is None else payload_raw.encode("utf-8")
    rc = _lib.tefas_query_operation_json_default(
        operation.encode("utf-8"),
        payload_arg,
        ctypes.byref(out),
        ctypes.byref(err),
    )
    if rc != 0:
        raise RuntimeError(_consume_ptr(err) or "tefas_query_operation_json_default failed")
    return json.loads(_consume_ptr(out))


__all__ = ["parse_document", "fetch_text", "query_operation"]
