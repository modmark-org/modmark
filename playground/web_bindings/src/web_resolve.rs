use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::atomic::Ordering::Release;

use js_sys::{ArrayBuffer, JsString};
use modmark_core::package_store::Resolve;
use modmark_core::package_store::{PackageSource, ResolveTask};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, Response, WorkerGlobalScope};

use crate::{read_file, recompile, REQUESTS_LEFT};

pub struct WebResolve;

impl Resolve for WebResolve {
    fn resolve_all(&self, paths: Vec<ResolveTask>) {
        REQUESTS_LEFT.fetch_add(paths.len(), Ordering::Release);
        paths.into_iter().for_each(resolve);
    }
}

thread_local! {
    // This should never fail if we are in a worker thread
    static WORKER_SCOPE: WorkerGlobalScope =
        js_sys::global()
            .dyn_into::<WorkerGlobalScope>()
            .unwrap();
}

static REGISTRY: OnceCell<Registry> = OnceCell::new();
static DEFAULT_REGISTRY: &str =
    "https://raw.githubusercontent.com/modmark-org/package-registry/main/package-registry.json";

#[derive(Error, Debug)]
pub enum WebResolveError {
    #[error("Invalid URL")]
    Url(String),
    #[error("Failed to get URL {0}: {1}")]
    Fetch(String, String),
    #[error("Failed to get registry from URL {0}: {1}")]
    FetchRegistry(String, String),
    #[error("Package {0} not in registry")]
    RegistryKey(String),
    #[error("Invalid registry JSON structure")]
    RegistryJSON,
    #[error("Local file doesn't exist: '{0}'")]
    File(String),
    #[error("This action is not implemented")]
    NotImplemented,
}

pub fn resolve(task: ResolveTask) {
    let target = task.package.source.clone();
    match target {
        PackageSource::Local => {
            spawn_local(async move {
                let file = fetch_local(&task.package.name);
                task.complete(file);
                request_done();
            });
        }
        PackageSource::Registry => {
            spawn_local(async move {
                let result = resolve_registry(&task.package.name, DEFAULT_REGISTRY).await;
                task.complete(result);
                request_done();
            });
        }
        PackageSource::Url => {
            spawn_local(async move {
                let result = resolve_url(&task.package.name).await;
                task.complete(result);
                request_done();
            });
        }
        PackageSource::Standard => {
            // This case can't occur
            unreachable!()
        }
    }
}

fn request_done() {
    if REQUESTS_LEFT.fetch_sub(1, Release) == 1 {
        // If previous value was 1 request left, we now have 0 requests left
        recompile();
    }
}

fn fetch_local(name: &str) -> Result<Vec<u8>, WebResolveError> {
    // Path must start at root
    let path = if name.starts_with('/') {
        name.to_string()
    } else {
        format!("/{name}")
    };
    let file = read_file(&path);
    if file.is_empty() {
        Err(WebResolveError::File(path))
    } else {
        Ok(file)
    }
}

async fn resolve_url(url: &str) -> Result<Vec<u8>, WebResolveError> {
    fetch_wasm_module(url).await
}

async fn resolve_registry(name: &str, url: &str) -> Result<Vec<u8>, WebResolveError> {
    let registry = if let Some(reg) = REGISTRY.get() {
        reg
    } else {
        let fetched = fetch_registry(url).await.map_err(|e| match e {
            WebResolveError::Fetch(a, b) => WebResolveError::FetchRegistry(a, b),
            x => x,
        })?;
        // If this fails, some previous thread already set the value
        // That is OK
        let _ = REGISTRY.set(fetched);
        REGISTRY.get().unwrap()
    };

    let entry = registry
        .0
        .get(name)
        .ok_or(WebResolveError::RegistryKey(name.to_string()))?;

    fetch_wasm_module(&entry.source).await
}

async fn fetch_wasm_module(source: &str) -> Result<Vec<u8>, WebResolveError> {
    fetch_bytes(source).await
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, WebResolveError> {
    let resp = fetch_url(url).await?;

    // This should not fail (no exceptions listed in MDN docs)
    let buffer_value: JsValue = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();

    // This should always succeed
    debug_assert!(buffer_value.is_instance_of::<ArrayBuffer>());

    // Any valid buffer should be castable into ArrayBuffer. Promise failures are caught earlier
    let buffer: ArrayBuffer = buffer_value.dyn_into().unwrap();

    // We can in turn turn this into an Uint8Array and then get a Vec<u8> from that
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

async fn fetch_registry(url: &str) -> Result<Registry, WebResolveError> {
    let resp = fetch_url(url).await?;

    // This should not fail (no exceptions listed in MDN docs)
    let content: JsValue = JsFuture::from(resp.text().unwrap()).await.unwrap();

    // This should always succeed
    debug_assert!(content.is_instance_of::<JsString>());
    let string = content.as_string().unwrap();

    // Try to parse registry
    serde_json::from_str(&string).map_err(|_| WebResolveError::RegistryJSON)
}

async fn fetch_url(url: &str) -> Result<Response, WebResolveError> {
    // Since this is interfacing with JS api:s, we have to use dynamic casting and refer to
    // API docs for knowing when it is safe or not. Comments will be added when appropriate.
    let mut opts = RequestInit::new();
    opts.method("GET");
    // Somehow, it doesn't work if we do opts.mode(RequestMode::Cors); for fetch_bytes, so I skip
    // it here as well

    // This only fails if we have credentials (user:password@url.com) in FF
    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|_| WebResolveError::Url(url.to_string()))?;

    // This only fails if we have an invalid header name
    request
        .headers()
        .set("Accept", "text/plain,application/json")
        .unwrap();

    // This doesn't fail on 404s, but does on invalid URL/headers/etc, see
    // https://developer.mozilla.org/en-US/docs/Web/API/fetch#exceptions
    let resp_value =
        JsFuture::from(WORKER_SCOPE.with(|w| w.fetch_with_request_and_init(&request, &opts)))
            .await
            .unwrap();

    // This should always succeed
    debug_assert!(resp_value.is_instance_of::<Response>());

    // Any valid response should be castable into Response. Promise failures are caught earlier
    let resp: Response = resp_value.dyn_into().unwrap();

    // If not 200, return early
    let status = resp.status();
    if status != 200 {
        return Err(WebResolveError::Fetch(
            url.to_string(),
            format!("Status code {status}"),
        ));
    }
    Ok(resp)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Registry(HashMap<String, RegistryEntry>);

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RegistryEntry {
    source: String,
}
