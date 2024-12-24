# TODO

## UI Features

- [ ] Add currentUser Context
- [ ] Add ability to edit and delete messages based on user context
- [ ] Make it so messages that are sent are sent with currentUser data
- [ ] Expand the messageStore internals to handle all stages of processing
- - [ ] Make it so messages are white to begin with but if they don't see a confirmation in 250ms then they become pending/grey
- [ ] Add user sidebar of offline and online users
- [ ] Add autoscrolling to new message if already at the bottom in MessageLogDisplay
- [ ] Add ability to jump to bottom in MessageLogDisplay

## Tauri

- [ ] Create comms and client-connect bindings in rust
- [ ] Design callback system for WS awaits and API calls
- [ ] Add Tauri browser support
