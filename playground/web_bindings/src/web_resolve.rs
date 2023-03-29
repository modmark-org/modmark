use std::convert::Infallible;
use std::future::Future;
use std::pin::{pin, Pin};
use std::sync::{Arc, Mutex};
use std::task::Poll;

use js_sys::{ArrayBuffer, Date, Map, Object, Uint8Array};
use modmark_core::Resolve;
use thiserror::Error;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Blob, ReadableStream, ReadableStreamDefaultReader, Request, RequestInit, RequestMode, Response, WorkerGlobalScope};

pub struct WebResolve;

#[derive(Debug, Error)]
#[error("Error fetching URL: {0}")]
pub struct FetchError(String);

impl Resolve for WebResolve {
    type Error = FetchError;

    fn resolve(&self, path: &str) -> Result<Vec<u8>, Self::Error> {
        web_sys::console::log_1(&("Making request".into()));
        let x = fetch_and_send_all(&vec![path]).pop().unwrap();
        web_sys::console::log_1(&("DONE!!!".into()));
        x

        /*if path.starts_with("https://") {
            spawn_fetcher_and_loader(path)
        } else {
            web_sys::console::log_1(&("Can only fetch HTTPS packages atm".into()));
            Ok(vec![])
        }*/
    }

    fn resolve_all(&self, paths: &[&str]) -> Vec<Result<Vec<u8>, Self::Error>> {
        paths.iter().map(|url| spawn_fetcher_and_loader(url)).collect()

        /*web_sys::console::log_1(&("Making request".into()));
        let x = fetch_and_send_all(paths);
        web_sys::console::log_1(&("DONE!!!".into()));
        x*/
    }
}

pub fn try_fetch() {
    spawn_local(async {
        let f1 = fetch_bytes("https://captive.apple.com/");
        let res = f1.await.unwrap();
        web_sys::console::log_1(&("DONE!!!".into()));
        web_sys::console::log_1(&(String::from_utf8(res).unwrap().into()));
    })
}

thread_local! {
    static WORKER_SCOPE: WorkerGlobalScope =
        js_sys::global()
            .dyn_into::<WorkerGlobalScope>()
            .unwrap();
}

fn spawn_fetcher_and_loader<T>(url: &str) -> Result<Vec<u8>, T> {
    web_sys::console::log_1(&("Spawn thingy".into()));

    let url = url.to_string();
    spawn_local(async move { fetch_and_load_module(url).await.unwrap(); });
    Ok(vec![])
}

fn fetch_and_send_all(paths: &[&str]) -> Vec<Result<Vec<u8>, FetchError>> {
    web_sys::console::log_1(&("Preparing fetch".into()));
    let (tx, rx) = std::sync::mpsc::channel();
    paths.iter().enumerate().for_each(|(id, path)| {
        web_sys::console::log_1(&("In iter".into()));
        let tx2 = tx.clone();
        let path2 = path.to_string();
        web_sys::console::log_1(&("Cloned".into()));
        spawn_local(async move {
            web_sys::console::log_1(&("Start fetch (BEFORE)".into()));
            let x = fetch_and_send(id, path2, tx2);
            web_sys::console::log_1(&("Start fetch (AFTER)".into()));
            x.await;
            web_sys::console::log_1(&("AFTER FETCH".into()));
        });
    });
    // We expect one rx per tx so collect them all
    let mut vec = (0..paths.len()).map(|_| rx.recv().unwrap()).collect::<Vec<(usize, Result<Vec<u8>, FetchError>)>>();
    vec.sort_by_key(|(idx, res)| *idx);
    vec.into_iter().map(|(_idx, res)| res).collect()
}

async fn fetch_and_send(id: usize, url: String, callback: std::sync::mpsc::Sender<(usize, Result<Vec<u8>, FetchError>)>) {
    web_sys::console::log_1(&("Fetching".into()));
    let result = fetch_bytes(&url).await.ok().ok_or(FetchError(format!("Could not load {url}")));
    web_sys::console::log_1(&("Fetched".into()));
    callback.send((id, result)).unwrap();
    web_sys::console::log_1(&("Sent".into()));
}

async fn fetch_and_load_module(url: String) -> Result<(), JsValue> {
    web_sys::console::log_1(&("Fetching".into()));
    let bytes = fetch_bytes(&url).await?;
    web_sys::console::log_1(&("Fetched".into()));
    crate::CONTEXT.with(|x| {
        loop {
            if let Ok(mut borrow) = x.try_borrow_mut() {
                web_sys::console::log_1(&("Inserted".into()));
                web_sys::console::log_1(&(format!("Bytes {:?}", bytes.as_slice()).into()));
                borrow.load_external_package(&url, bytes.as_slice()).unwrap();
                borrow.get_all_package_info().iter().for_each(|pkg| {
                    web_sys::console::log_1(&(format!("Package {}", pkg.name).into()));
                });
                break;
            }
            web_sys::console::log_1(&("Not Inserted".into()));
        }
    });
    Ok(())
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, JsValue> {
    web_sys::console::log_1(&(format!("GET TO {url}").into()));
    let mut opts = RequestInit::new();
    opts.method("GET");
    //opts.mode(RequestMode::NoCors);

    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("Accept", "application/octet-stream").unwrap();
    let worker_scope: WorkerGlobalScope =
        js_sys::global()
            .dyn_into::<WorkerGlobalScope>()
            .unwrap();
    web_sys::console::log_1(&(format!("{:?}", &request).into()));
    let resp_value = JsFuture::from(worker_scope.fetch_with_request_and_init(&request, &opts)).await?;
    assert!(resp_value.is_instance_of::<Response>());
    let resp: Response = resp_value.dyn_into().unwrap();
    let buffer: ArrayBuffer = JsFuture::from(resp.array_buffer()?).await?.dyn_into().unwrap();
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
    /*
    let stream: ReadableStream = resp.body().unwrap();
    let reader: ReadableStreamDefaultReader = stream.get_reader().dyn_into().unwrap();
    let mut array: Uint8Array = Uint8Array::new_with_length(0);
    let final_array = loop {
        let obj = JsFuture::from(reader.read()).await?;
        let done: bool = js_sys::Reflect::get(&obj, &("done".into())).unwrap().as_bool().unwrap();
        if done { break array; }
        let value: Uint8Array = js_sys::Reflect::get(&obj, &("value".into())).unwrap().dyn_into().unwrap();
        let new_array = Uint8Array::new_with_length(array.length() + value.length());
        new_array.set(&array, 0);
        new_array.set(&value, array.length());
        array = new_array;
    };

    let array = final_array.to_vec();
    Ok(array)
     */
}

#[wasm_bindgen(module = "/src/fetch_file.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn fetch_file(url: String) -> Result<JsValue, JsValue>;
}

fn sleep(ms: f64) {
    let target = Date::new_0().get_time() + ms;
    while Date::new_0().get_time() <= target {}
}
