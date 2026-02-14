use std::sync::mpsc::{Receiver, SendError, Sender, TryRecvError};

use crate::device::{DeviceMessage, DeviceRecieverTrait, DeviceSenderTrait};

pub type NativeHostSender = Sender<DeviceMessage>;
pub type NativeHostReciever = Receiver<DeviceMessage>;

pub struct NativeSender {
    sender: Option<Sender<DeviceMessage>>,
}

pub struct NativeReciever {
    reciever: Option<Receiver<DeviceMessage>>,
}

impl Default for NativeSender {
    fn default() -> Self {
        Self { sender: None }
    }
}

impl NativeSender {
    pub fn new(sender: Sender<DeviceMessage>) -> Self {
        Self {
            sender: Some(sender),
        }
    }
}

impl Default for NativeReciever {
    fn default() -> Self {
        Self { reciever: None }
    }
}

impl NativeReciever {
    pub fn new(reciver: Receiver<DeviceMessage>) -> Self {
        Self {
            reciever: Some(reciver),
        }
    }
}

impl DeviceSenderTrait for NativeSender {
    type E = SendError<DeviceMessage>;

    fn send_to_host(&mut self, message: DeviceMessage) -> Result<(), Self::E> {
        self.sender.as_ref().unwrap().send(message)
    }
}

impl DeviceRecieverTrait for NativeReciever {
    type E = TryRecvError;

    fn try_recv_from_host(&self) -> Result<DeviceMessage, Self::E> {
        self.reciever.as_ref().unwrap().try_recv()
    }
}
