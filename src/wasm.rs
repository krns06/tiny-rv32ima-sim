use wasm_bindgen::{JsCast, prelude::wasm_bindgen};
use web_sys::{CanvasRenderingContext2d, Window, console, window};

use crate::simulator::{self, Simulator, WasmLoaded};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace=console)]
    pub fn log(s: &str);

    #[wasm_bindgen]
    pub fn append_console(c: u8);
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
}
