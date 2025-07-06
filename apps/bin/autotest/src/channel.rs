use starina::channel::Channel;
use starina::channel::RecvError;
use starina::error::ErrorCode;
use starina::handle::Handleable;
use starina::message::Message;
use starina::message::MessageBuffer;
use starina::poll::Poll;
use starina::poll::Readiness;

pub fn test_channel() {
    let (ch1, ch2) = Channel::new().unwrap();
    let poll1 = Poll::new().unwrap();
    poll1
        .add(
            ch1.handle_id(),
            (),
            Readiness::READABLE | Readiness::WRITABLE,
        )
        .unwrap();

    let poll2 = Poll::new().unwrap();
    poll2
        .add(
            ch2.handle_id(),
            (),
            Readiness::READABLE | Readiness::WRITABLE,
        )
        .unwrap();

    // A newly created channel is already writable.
    assert_eq!(poll1.try_wait().map(|x| x.1), Ok(Readiness::WRITABLE));

    // Send a message to the channel.
    let result = ch1.send(Message::Data { data: b"" });
    assert_eq!(result, Ok(()));

    // The peer channel is now readable.
    assert_eq!(
        poll2.try_wait().map(|x| x.1),
        Ok(Readiness::READABLE | Readiness::WRITABLE)
    );

    // Fill the channel until it's full.
    loop {
        let result = ch1.send(Message::Data { data: b"" });
        match result {
            Ok(_) => {}
            Err(err) if err == ErrorCode::Full => {
                break;
            }
            Err(err) => {
                panic!("unexpected error: {}", err);
            }
        }
    }

    // The channel is now full. It's no longer writable.
    assert_eq!(poll1.try_wait().map(|x| x.1), Err(ErrorCode::WouldBlock));

    // Receive a message from the peer channel.
    let mut msgbuffer = MessageBuffer::new();
    let result = ch2.recv(&mut msgbuffer);
    assert!(matches!(result, Ok(Message::Data { data: b"" })));

    // The channel is now writable.
    assert_eq!(poll1.try_wait().map(|x| x.1), Ok(Readiness::WRITABLE));

    // Drain the peer channel.
    loop {
        let result = ch2.recv(&mut msgbuffer);
        match result {
            Ok(_) => {}
            Err(RecvError::Syscall(err)) if err == ErrorCode::Empty => {
                break;
            }
            Err(err) => {
                panic!("unexpected error: {:?}", err);
            }
        }
    }

    // The channel is now writable.
    assert_eq!(poll1.try_wait().map(|x| x.1), Ok(Readiness::WRITABLE));
    // The peer channel has no messages to receive.
    assert_eq!(poll2.try_wait().map(|x| x.1), Ok(Readiness::WRITABLE));
}
