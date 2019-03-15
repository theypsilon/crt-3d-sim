use crate::wasm_error::WasmResult;
use js_sys::{Float32Array, Int32Array};
use web_sys::Window;

pub fn window() -> WasmResult<Window> {
    Ok(web_sys::window().ok_or("cannot access window")?)
}

pub fn now() -> WasmResult<f64> {
    Ok(window()?.performance().ok_or("cannot access performance")?.now())
}

pub fn js_f32_array(data: &[f32]) -> Float32Array {
    let array = Float32Array::new(&wasm_bindgen::JsValue::from(data.len() as u32));
    for (i, f) in data.iter().enumerate() {
        array.fill(*f, i as u32, (i + 1) as u32);
    }
    array
}

pub fn js_i32_array(data: &[i32]) -> Int32Array {
    let array = Int32Array::new(&wasm_bindgen::JsValue::from(data.len() as u32));
    for (i, f) in data.iter().enumerate() {
        array.fill(*f, i as u32, (i + 1) as u32);
    }
    array
}
