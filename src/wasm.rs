use std::mem::transmute;

use wasm_bindgen::{Clamped, JsCast, prelude::wasm_bindgen};
use web_sys::{CanvasRenderingContext2d, ImageData, Window, console, window};

use crate::{
    device::{DeviceMessage, DeviceRecieverTrait, DeviceSenderTrait},
    host_device::{GpuMessage, GpuOperation},
    simulator::{self, Simulator, WasmLoaded},
};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=console)]
    pub fn log(s: &str);

    #[wasm_bindgen]
    pub fn append_console(c: u8);
}

pub struct WasmGpuSender {
    canvas_ctx: Option<CanvasRenderingContext2d>,
}

#[derive(Default)]
pub struct WasmUartReciever {}

impl Default for WasmGpuSender {
    fn default() -> Self {
        Self { canvas_ctx: None }
    }
}

impl DeviceSenderTrait for WasmGpuSender {
    type E = ();

    fn send_to_host(&mut self, message: crate::device::DeviceMessage) -> Result<(), Self::E> {
        if let DeviceMessage::Gpu(message) = message {
            if message.operation == GpuOperation::Copy {
                let size = message.buffer.len() * 4;
                let buffer: &[u8] = unsafe {
                    std::slice::from_raw_parts(message.buffer.as_ptr() as *const u8, size)
                };

                let image_data = ImageData::new_with_u8_clamped_array_and_sh(
                    Clamped(buffer),
                    message.rect.width,
                    message.rect.height,
                )
                .unwrap();
                let x = message.rect.x as f64;
                let y = message.rect.y as f64;

                let canvas_ctx = self.canvas_ctx.as_ref().unwrap();

                canvas_ctx
                    .put_image_data(&image_data, x as f64, y as f64)
                    .unwrap();
            }
        }

        Ok(())
    }
}

impl WasmGpuSender {
    pub fn new(canvas_ctx: CanvasRenderingContext2d) -> Self {
        Self {
            canvas_ctx: Some(canvas_ctx),
        }
    }
}

impl DeviceRecieverTrait for WasmUartReciever {
    type E = ();

    fn try_recv_from_host(&self) -> Result<DeviceMessage, Self::E> {
        Err(())
    }
}

#[wasm_bindgen]
pub struct WasmSimulator {
    simulator: Simulator<WasmLoaded>,
}

#[wasm_bindgen]
impl WasmSimulator {
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: &str) -> Self {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document
            .get_element_by_id(canvas_id)
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        let mut simulator = Simulator::new().setup_wasm_devices(context);

        let buf = include_bytes!("../statics/fw_jump.bin");
        simulator.load_flat(buf, 0x80000000);

        let buf = include_bytes!("../statics/platform.dtb");
        simulator.load_flat(&buf.as_slice(), 0x80100000);

        let buf = include_bytes!("../statics/Image");
        simulator.load_flat(&buf.as_slice(), 0x80400000);

        Self {
            simulator: simulator.set_entry_point(0x80000000),
        }
    }

    pub fn step(&mut self) {
        for _ in 0..3000000 {
            self.simulator.step();
        }
    }

    pub fn send_key(&mut self, key: u8) {
        self.simulator.send_key(key as char);
    }
}
