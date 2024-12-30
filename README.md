# nerdtalk

![CI](https://github.com/ethanuppal/nerdtalk/actions/workflows/ci.yaml/badge.svg)
![Code Style](https://github.com/ethanuppal/nerdtalk/actions/workflows/clippy.yaml/badge.svg)

## Running locally

First, you'll want to generate local certificates

```sh
# mark scripts as executable
chmod u+x scripts/*.sh 

# generate local testing certificates
./scripts/gen_cert.sh

# in a new shell, run the server
./scripts/local_server.sh

# in the current shell, run the client
./scripts/local_tui_client.sh

# in yet another shell, run another client
./scripts/local_tui_client.sh
```
You can now talk to each other over the TUI interface!

## Running on nerdserver

to be figured out!
