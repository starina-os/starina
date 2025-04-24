# Channel

Channel is the main Inter-Process Communication (IPC) mechanism in Starina. Channel is:

- A bounded message queue, where each message contains a bytes array and a handles array.
- Connected to its peer channel. Messages are delivered to the peer channel.
- Asynchronous and non-blocking. If the queue is full, it immediately returns an error.
- Movable between processes.

## Message Types

Unlike other IPC systems like gRPC, Fuchsia's FIDL, or Mach's MIG, Starina does not use Interface Definition Language (IDL) to define message types. Instead, we use few pre-defined message types:

| Name | Parameters | Context | Description |
|------|------------|-------------|--------|
| `Connect` | `handle: Channel` | listen channel | A new connection to the server. `handle` is a new control channel connected to the client. |
| `Open` | `call_id: i32, uri: string` | control channel | Open a resource, such as a file or a TCP socket. |
| `OpenReply` | `handle: Channel` | control channel | Reply to an `Open` request with a new data channel. `handle` will be moved to the destination channel. |
| `Abort` | `call_id: i32, error: i32` | control channel | Reply to an `Open` request when it fails. `error` is the error code. |
| `FramedData` | `data: bytes` | data channel | A frame of data, such as network packets. |
| `StreamData` | `data: bytes` | data channel | A stream of data, such as file contents or TCP data. |
| `Error` | `error: i32` | data channel | An error occurred in the data channel, like no space left on the disk. |

### Key Points

- IDL is not used to keep things simple, sacrificing some type safety and flexibility. This is similar to UNIX's *everything is a file* philosophy. Starina does the similar thing, but in a message-oriented way.
- `Open` and `OpenReply` contain `call_id`, which is a unique identifier for the request so that multiple requests can be handled at the same time.
- `Connect`, `FramedData`, and `StreamData` are so-called *fire-and-forget* messages. Errors are reported through the `Error` message asynchronously. This is similar to Node.js' API design where you register a callback to handle `error` events.

> [!TIP]
>
> `Close` message is not defined because you can simply close the channel.

### Upcoming Types (not implemented yet)

More message types are being considered. Here are some of them to stimulate your imagination:

| Name | Parameters | Context | Description |
|------|------------|-------------|--------|
| `Invoke` | `call_id: i32, method: string, args: bytes` | control channel | Invoke a method on a resource. It's a generic RPC call similar to `ioctl(2)` in UNIX. |
| `InvokeReply` | `call_id: i32, result: bytes` | control channel | Reply to an `Invoke` request. |
| `Pipe` | `call_id: i32, uri: string, write_to: Channel` | control channel | Open a resource, write `FramedData`/`StreamData` to `write_to`, and reply with `PipeReply` once it's done. The channel `write_to` will be cloned with only the sender right. This is similar to Linux's `sendfile(2)`. |
| `PipeReply` | `written: usize` | control channel | Reply to a `Pipe` request. |
