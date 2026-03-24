#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct RendererInfo {
    adapter_name: String,
    backend: String,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl RendererInfo {
    #[wasm_bindgen(getter)]
    pub fn adapter_name(&self) -> String {
        self.adapter_name.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn backend(&self) -> String {
        self.backend.clone()
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn probe_webgpu() -> Result<RendererInfo, JsValue> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let info = adapter.get_info();

    adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|error| JsValue::from_str(&error.to_string()))?;

    Ok(RendererInfo {
        adapter_name: info.name,
        backend: format!("{:?}", info.backend),
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn native_build_note() -> &'static str {
    "render-wasm is intended for the wasm32-unknown-unknown target"
}
