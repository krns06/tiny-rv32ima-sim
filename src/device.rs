use std::fmt::Debug;

use crate::{IRQ, host_device::GpuMessage, memory::Memory};

pub type DeviceResult<T> = crate::Result<DeviceResponse<T>>;

pub struct DeviceResponse<T> {
    pub value: T,
    pub is_interrupting: bool,
}

// 仮想デバイスとホストデバイスとの通信に使用する列挙体
pub enum DeviceMessage {
    Uart(char),
    Net(Vec<u8>),
    Gpu(GpuMessage),
    None,
}

// 仮想デバイスからホストデバイスに対してのsenderに関してのトレイト
pub trait DeviceSenderTrait: Default {
    type E: Debug;

    fn send_to_host(&mut self, message: DeviceMessage) -> Result<(), Self::E>;
}

// 仮想デバイスがホストデバイスからのメッセージを受け取るためのトレイト
pub trait DeviceRecieverTrait: Default {
    type E: Debug;

    fn try_recv_from_host(&self) -> Result<DeviceMessage, Self::E>;
}

// 仮想デバイスについてのトレイト
pub trait DeviceTrait {
    fn read(&mut self, offset: u32, size: u32, memory: &mut Memory) -> DeviceResult<u32>;
    fn write(
        &mut self,
        offset: u32,
        size: u32,
        value: u32,
        memory: &mut Memory,
    ) -> DeviceResult<()>;

    fn irq(&self) -> IRQ;

    // 割り込みが起こったときのみ行う必要があるもののフラグの切り替えに使用する関数
    fn take_interrupt(&mut self) {}

    // イベントループからメッセージをデバイスに通知するときに使われる関数
    // そのデバイス向けではない場合は受け取るべきではない。
    #[cfg(target_arch = "wasm32")]
    fn handle_incoming(&mut self, message: &DeviceMessage) {}

    // tickごとに実行される関数
    // 外部割り込みが有効な場合に実行される
    fn tick(&mut self, _: &mut Memory) -> bool {
        false
    }
}
