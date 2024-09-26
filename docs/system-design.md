# 1 Concepts

## 1.1 Device

A device represents a single session for a user, which holds some different properties (like `local_pts`). When a user received some messages, they are broadcasted to all online devices.

## 1.2 Connection

User devices connect to the server with connections. It's a WebSocket stream, in where JSON data are transported. The relationship between device and connection is 1:1, which means a device will only have one connection at the same time.

## 1.3 Mailbox

Mailbox is a bounded queue that contains a series of messages for a user. The user must receive messages from its mailbox in order. The mailbox may drop oldest message when it's full, therefore we need store persistent data in other places (such as database).

Every message in the mailbox has an unique ID called `pts`, which is derived based on timestamps (usually milliseconds elapsed from 2024-01-01 00:00:00). The client maintains a value called `local_pts` to record the ID of the latest message it received. When `local_pts` is not matched with remote `pts`, a **sync operation** is required, which will be explained in further sections.

## 1.4 Push Notification

When a connection is established, the server can send push notifications to clients on it. Push notifications hold important data that clients need to process, like new messages. The mailbox will store a window of push notifications as updates to be received by devices that are not online at the time.

Clients must handle every notification it receive, since server won't resent them once they've been received.

Before subscribing push notifications, the client must perform sync operation to sync its state with the server. The subscription will be created automatically when the sync operation is done.

# 2 Sync Protocol

## 2.1 Introduction

The sync protocol is the core of the whole system. It helps user data keep up-to-date across all devices. Each time a device gets online, it needs to update its state with the sync protocol before subscribing push notifications.

As described earlier, mailbox plays an important role in the sync protocol. Updates are sent to mailboxes and will be received by user devices later. The sync protocol defines how the user devices should fetch data and interact with the server.

Sync operations and push notifications both perform in the same connection, but push notifications are suspended before the sync operation ends.

## 2.2 Initiate a Sync Operation

After logging in, the client can initiate a sync operation with `sync` command:

```json
{
  "cmd": "sync",
  "local_pts": 42
}
```

The server will return the update list with an event:

```json
{
  "event": "sync_updates",
  "updates": [
    // ...
  ],
  "pts": 50,
  "too_long": false,
  "up_to_date": false
}
```

When the client receives this event, it should handle these updates and set its `local_pts` to `pts` of the event. `up_to_date` field indicates whether the client is synced with the server.

If `update_to_date` is `true`, it means the client is ready to receive push notifications, otherwise we need to repeat this process.

## 2.3 Handle "too long" Updates

When a device is offline for a long time, it may receive `sync_updates` event with `too_long` field set. This means the server has dropped some updates that the device didn't receive, and applying those updates will create a gap between server and client.

Clients may have several ways to handle it:

1. Clear the local database and apply the updates. The client can fetch the history messages later.
2. Add a mark to the last message and apply the updates. When user scrolls to that message, the client can fetch history from the server to fill the gap. But be aware that the local messages may be stale, e.g., have been deleted.

The sync protocol doesn't enforce clients' behavior of handling "too long" updates, but clients should take care of the user experience when it happens, and ensure no messages will be lost locally.
