use clap::ValueEnum;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use tefas::runtime::BlockingClient;
use tefas::{AppConfig, Operation};

fn c_ptr_to_str(ptr: *const c_char) -> anyhow::Result<String> {
    if ptr.is_null() {
        return Err(anyhow::anyhow!("null string pointer"));
    }
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| anyhow::anyhow!("input is not valid UTF-8"))?;
    Ok(s.to_string())
}

fn into_c_string_ptr(input: String) -> *mut c_char {
    CString::new(input)
        .expect("ffi output must not contain NUL bytes")
        .into_raw()
}

fn set_out_ptr(dst: *mut *mut c_char, value: String) {
    if !dst.is_null() {
        unsafe { *dst = into_c_string_ptr(value) };
    }
}

fn set_err(out_err: *mut *mut c_char, msg: String) -> i32 {
    set_out_ptr(out_err, msg);
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn tefas_version() -> *mut c_char {
    into_c_string_ptr(env!("CARGO_PKG_VERSION").to_string())
}

#[unsafe(no_mangle)]
/// # Safety
///
/// `ptr` must be a pointer previously returned by this library via
/// `CString::into_raw` and must not have been freed already.
pub unsafe extern "C" fn tefas_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tefas_parse_document_json(
    html: *const c_char,
    out_json: *mut *mut c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let html = match c_ptr_to_str(html) {
        Ok(v) => v,
        Err(e) => return set_err(out_err, e.to_string()),
    };

    let (grouped, meta) = tefas::parse_document(&html);
    let payload = serde_json::json!({
        "data": grouped,
        "meta": meta,
    });

    match serde_json::to_string(&payload) {
        Ok(serialized) => {
            set_out_ptr(out_json, serialized);
            0
        }
        Err(e) => set_err(out_err, format!("serialize error: {e}")),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tefas_fetch_text_default(
    url: *const c_char,
    out_text: *mut *mut c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let url = match c_ptr_to_str(url) {
        Ok(v) => v,
        Err(e) => return set_err(out_err, e.to_string()),
    };

    let cfg = AppConfig::with_defaults();
    let client = match BlockingClient::new(&cfg) {
        Ok(c) => c,
        Err(e) => return set_err(out_err, format!("client init error: {e}")),
    };

    match client.fetch_text(&url) {
        Ok(text) => {
            set_out_ptr(out_text, text);
            0
        }
        Err(e) => set_err(out_err, format!("fetch error: {e}")),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tefas_query_operation_json_default(
    operation_name: *const c_char,
    payload_json: *const c_char,
    out_json: *mut *mut c_char,
    out_err: *mut *mut c_char,
) -> i32 {
    let op_name = match c_ptr_to_str(operation_name) {
        Ok(v) => v,
        Err(e) => return set_err(out_err, e.to_string()),
    };

    let op = match Operation::value_variants().iter().copied().find(|op| {
        op.to_possible_value()
            .map(|v| v.matches(&op_name, true))
            .unwrap_or(false)
    }) {
        Some(op) => op,
        None => return set_err(out_err, format!("unknown operation: {op_name}")),
    };

    let payload = if payload_json.is_null() {
        op.default_payload()
    } else {
        let raw = match c_ptr_to_str(payload_json) {
            Ok(v) => v,
            Err(e) => return set_err(out_err, e.to_string()),
        };
        match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => v,
            Err(e) => return set_err(out_err, format!("invalid payload json: {e}")),
        }
    };

    let cfg = AppConfig::with_defaults();
    let client = match BlockingClient::new(&cfg) {
        Ok(c) => c,
        Err(e) => return set_err(out_err, format!("client init error: {e}")),
    };

    let spec = op.spec();
    let url = if spec.endpoint.starts_with("http") {
        spec.endpoint.to_string()
    } else {
        format!("{}{}", cfg.normalized_base_url(), spec.endpoint)
    };

    match client.post_json_with_referer(&url, spec.referer, &payload) {
        Ok(resp) => match serde_json::to_string(&resp) {
            Ok(s) => {
                set_out_ptr(out_json, s);
                0
            }
            Err(e) => set_err(out_err, format!("serialize error: {e}")),
        },
        Err(e) => set_err(out_err, format!("query error: {e}")),
    }
}
