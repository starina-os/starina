use starina_types::{error::ErrorCode, message::MessageBuffer};

#[derive(Debug)]
pub struct ChannelSender {}

#[derive(Debug)]
pub struct ChannelReceiver {}

impl ChannelReceiver {
    pub fn try_recv<'a, M>(
        &self,
        buffer: &'a mut MessageBuffer,
    ) -> Result<Option<M>, ErrorCode> {
        todo!()
    }
}
