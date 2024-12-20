# Protocol V0

Support read chat logs + appending to chat log

## V0

Eventually every client agrees on messages up to a given message, but some
clients may be ahead (because of network connectivity or later login)

### Reading Messages
- timer fires periodically
- sends the server the latest message sequence # it's seen
- server replies with all the missing messages

### Appending Messages
- when a client sends a message, a thread for that session puts it into spmc
- for every new connection to the session, you create a fresh sequence number (set to 0). The sequence number is the order in which you send your next message relative to the global message list. This increments every message you send
- local thread delivery rule:
    - When the thread receives a sequence number n, and you haven't received up to n-1, you receive the difference up to n-1, then you send the message (via the server) into the spmc
    - This ensures consistency between client and server
- Server maintains a list of all spmcs for the session (pulls at the start), and has at most 1 message from each session in a list. The only required ordering is by the timestamp of the messages within this list. When you choose a message, you pull the receiver again. Commit this message to the next slot number, and increment the server slot number.

Assumption: Stable storage, server never crashes, clients follow protocol :)

## V0.1

### Definitions

Chat Entry (slot)

### Messages

#### Client
- **ClientAppend**: Append a new chat entry (sends message w/ metadata)
- **ClientUpdate**: Request $n$ slots up to a given slot number $N$
    - If $N = -1$ then request up to current slot number

#### Server
- **ServerAck**: Ack to tell the client that it's committed the chat log entry
- **ServerReply**: Response contained all requested chat entries and latest slot number
    - If requested message number is > sendable amount, send the min of the two

### Algorithm 

TCP gives reliable FIFO order for messages.

#### Client

- On startup (login/recovery/reconnect are all the same bc we're not caching atm) we send out a ClientUpdate for the last $m$ slots
- When the user inputs a message, send ClientAppend to server
- When you scroll past the last sequence number you possess locally, send ClientUpdate

#### Server
- websockets for each client connection are directly connected to mpscs
- server has an infinite loop where it (attempts to) poll the channel
    - On ClientAppend, write this to stable storage (db) then send a ServerAck to all connections
    - On ClientUpdate, just do it what it says :3
    
