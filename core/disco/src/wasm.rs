
use wasm_bindgen::prelude::*;

use crate::{error::DiscoveryError};

fn setup_ws(url: &str) -> Result<web_sys::WebSocket, DiscoveryError> {
    use wasm_bindgen::{prelude::Closure, JsCast};
    use web_sys::MessageEvent;
    let ws = web_sys::WebSocket::new(url)
        .map_err(|e| DiscoveryError::OpenSocketError(e.as_string().unwrap()))?;

    let onopen_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
        use web_sys::js_sys;
        if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            use crate::utils::handle_message;

            tracing::info!("message event, received arraybuffer: {:?}", abuf);
            let array = js_sys::Uint8Array::new(&abuf);
            let len = array.byte_length() as usize;
            tracing::info!("Arraybuffer received {}bytes", len);
            if let Err(err) = handle_message(&array.to_vec()) {
                tracing::error!("{:?}", err);
            }
        }
    });
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();

    let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::ErrorEvent| {
        tracing::info!("ws error: {:?}", e);
    });
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();

    Ok(ws)
}

#[wasm_bindgen]
pub struct DiscoClient {
    ws: Option<web_sys::WebSocket>
}

#[wasm_bindgen]
impl DiscoClient {
}
