use starina_api::channel::CallError;
use starina_api::types::message::MessageBuffer;
use starina_api::types::message::MessageSerialize;

use crate::helpers::Context;
use crate::starina_autogen::idl::echo::OhSnapError;
use crate::starina_autogen::idl::echo::Ping;
use crate::starina_autogen::idl::echo::PleaseFail;

pub fn test_channel_call(ctx: &mut Context) {
    let mut msgbuffer = MessageBuffer::new();
    let reply = ctx.echo.call(&mut msgbuffer, Ping { value: 123 }).unwrap();
    assert_eq!(reply.value, 123);
}

pub fn test_error_reply(ctx: &mut Context) {
    let mut msgbuffer = MessageBuffer::new();
    let result = ctx.echo.call(&mut msgbuffer, PleaseFail {});
    assert_eq!(result, Err(CallError::Unexpected(OhSnapError::MSGINFO)));
}
