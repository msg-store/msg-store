use crate::store::ByteSize;
use crate::uuid::Uuid;
use std::rc::Rc;

#[derive(Debug)]
pub struct MsgData {
    uuid: Rc<Uuid>,
    byte_size: ByteSize,
}

impl MsgData {
    pub fn new(uuid: Rc<Uuid>, byte_size: ByteSize) -> MsgData {
        MsgData {
            uuid: uuid.clone(),
            byte_size,
        }
    }

    pub fn get_uuid(&self) -> Rc<Uuid> {
        self.uuid.clone()
    }

    pub fn get_byte_size(&self) -> ByteSize {
        self.byte_size
    }
}

#[cfg(test)]
mod tests {}
