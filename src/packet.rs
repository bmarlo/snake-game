pub const PROTOCOL_ID: u64 = 0xaefdb87fe753ba07;
pub const HEADER_SIZE: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Opcode {
    Sync = 0x01,
    NewDirection,
    NewTarget
}

pub struct Packet {
    opcode: Opcode,
    data: Vec<u8>
}

impl Packet {
    pub fn new(opcode: Opcode, size: usize) -> Packet {
        Packet { opcode, data: Vec::with_capacity(size) }
    }

    pub fn push_data(&mut self, data: &[u8]) {
        if self.data.len() + data.len() > u16::MAX as usize {
            panic!("bad data size [Packet::push_data()]");
        }

        self.data.extend_from_slice(data);
    }

    pub fn opcode(&self) -> Opcode {
        self.opcode
    }

    pub fn data(&self) -> &Vec<u8> {
        &self.data
    }

    pub fn encode(&self) -> Vec<u8> {
        let size = HEADER_SIZE + self.data.len();
        let mut buffer = Vec::with_capacity(size);

        buffer.push((PROTOCOL_ID >> 56) as u8);
        buffer.push((PROTOCOL_ID >> 48) as u8);
        buffer.push((PROTOCOL_ID >> 40) as u8);
        buffer.push((PROTOCOL_ID >> 32) as u8);
        buffer.push((PROTOCOL_ID >> 24) as u8);
        buffer.push((PROTOCOL_ID >> 16) as u8);
        buffer.push((PROTOCOL_ID >> 8) as u8);
        buffer.push((PROTOCOL_ID >> 0) as u8);

        buffer.push((self.opcode as u16 >> 8) as u8);
        buffer.push((self.opcode as u16 >> 0) as u8);

        let size = self.data.len();
        buffer.push((size >> 8) as u8);
        buffer.push((size >> 0) as u8);

        buffer.extend_from_slice(&self.data);
        buffer
    }

    pub fn decode(buffer: &[u8]) -> Option<Packet> {
        if buffer.len() < HEADER_SIZE {
            return None;
        }

        let mut protocol_id: u64 = 0;
        protocol_id |= (buffer[0] as u64) << 56;
        protocol_id |= (buffer[1] as u64) << 48;
        protocol_id |= (buffer[2] as u64) << 40;
        protocol_id |= (buffer[3] as u64) << 32;
        protocol_id |= (buffer[4] as u64) << 24;
        protocol_id |= (buffer[5] as u64) << 16;
        protocol_id |= (buffer[6] as u64) << 8;
        protocol_id |= (buffer[7] as u64) << 0;

        if protocol_id != PROTOCOL_ID {
            return None;
        }

        let mut opcode: u16 = 0;
        opcode |= (buffer[8] as u16) << 8;
        opcode |= (buffer[9] as u16) << 0;

        let opcode = match opcode {
            0x01 => {
                Opcode::Sync
            },
            0x02 => {
                Opcode::NewDirection
            },
            0x03 => {
                Opcode::NewTarget
            },
            _ => {
                return None;
            }
        };

        let mut size: u16 = 0;
        size |= (buffer[10] as u16) << 8;
        size |= (buffer[11] as u16) << 0;

        if size as usize != buffer.len() - HEADER_SIZE {
            return None;
        }

        let mut packet = Packet::new(opcode, size as usize);
        packet.data.extend_from_slice(&buffer[HEADER_SIZE..]);
        Some(packet)
    }
}
