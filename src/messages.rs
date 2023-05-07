use bitfield::bitfield;

#[derive(Debug)]
pub enum ControlMessage {
    Data = 0,
    Open = 1,
    OpenAck = 2,
    Close = 3,
}

impl From<u8> for ControlMessage {
    fn from(value: u8) -> Self {
        match value {
            0 => ControlMessage::Data,
            1 => ControlMessage::Open,
            2 => ControlMessage::OpenAck,
            3 => ControlMessage::Close,
            _ => {
                todo!()
            }
        }
    }
}

impl From<ControlMessage> for u8 {
    fn from(value: ControlMessage) -> Self {
        match value {
            ControlMessage::Data => 0,
            ControlMessage::Open => 1,
            ControlMessage::OpenAck => 2,
            ControlMessage::Close => 3,
        }
    }
}

bitfield! {
    pub struct Header(u64);
    pub u8, from into ControlMessage, control, set_control: 1, 0;
    pub u32, stream_id, set_stream_id: 31, 2;
    pub u32, data_size, set_data_size: 63, 32;
}

impl Header {
    pub fn new(control: ControlMessage, stream_id: u32, data_size: u32) -> Header {
        let mut header = Header(0);
        header.set_control(control);
        header.set_stream_id(stream_id);
        header.set_data_size(data_size);

        header
    }
}