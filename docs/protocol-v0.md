# Protocol V0

Support read chat logs + appending to chat log

## Guarantees

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