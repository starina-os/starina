# Interface Spec

## Overview

Starina provides a message passing based inter-process communication mechanism called channels. Using channels, apps can send arbitrary data and kernel handles to each other, across the isolation boundary.

## Client-Server Model

An implicit assumption here is we can categorize userspace programs into two categories: *clients* and *servers*, akin to HTTP: network device driver server provides `ethernet_device` service, TCP/IP server provides `tcpip` service, for example.

Interface spec standardizes the message format and the way to define service methods (or RPCs). The build system automatically generates useful stub codes for serialization/deserilization, and the user can focus on the business logic.

It's similar to gRPC or OpenAPI in HTTP.

> [!TIP]
>
> **Design decision: Why not using serde?**
>
> This is because we want to make Starina language-agnostic. Also, defining interfaces in JSON allows third-party tools to parse definitions easily.

## Example

Let's look at an example of an interface definition for the `echo` interface:

```json
{
  "name": "echo",
  "kind": "interface/v0",
  "spec": {
    "messages": [
      {
        "name": "ping",
        "context": "control",
        "type": "call",
        "params": [
          {
            "name": "value",
            "type": "int32",
            "help": "The value to return"
          }
        ],
        "returns": [
          {
            "name": "value",
            "type": "int32",
            "help": "The value returned"
          }
        ]
      }
    ]
  }
}
```

## Fields

| Field | Description| Example |
| --- | --- | --- |
| `name` | The interface name. | `echo` |
| `kind` | Must be `interface/v0`. | |
| `spec.messages` | The list of messages. | |

### Messages

`spec.messages` is a list of messages, more specifically, the list of RPCs that the service provides.

| Field | Description | Example |
| --- | --- | --- |
