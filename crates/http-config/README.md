# http-config

Generic HTTP/TLS/Retry config tipleri.

## Saglanan tipler

- `HttpBackend`
- `TlsBackend`
- `TlsConfig`
- `RetryConfig`
- `DEFAULT_TIMEOUT_SECS`
- `DEFAULT_RETRY_COUNT`
- `DEFAULT_RETRY_BACKOFF_MS`

TEFAS veya KAP'a ozel default URL/referer/user-agent bu crate'te bulunmaz.

## Build

```bash
cd shared
cargo check -p http-config
```
