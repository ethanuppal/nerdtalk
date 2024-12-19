# nerdtalk

![CI](https://github.com/ethanuppal/nerdtalk/actions/workflows/ci.yaml/badge.svg)
![Code Style](https://github.com/ethanuppal/nerdtalk/actions/workflows/clippy.yaml/badge.svg)

## Running locally

First, you'll want to generate local certificates

```sh
# mark scripts as executable
chmod u+x ./gen_cert.sh
chmod u+x ./client.sh
chmod u+x ./server.sh

# generate local testing certificates
./gen_cert.sh

# in a new shell, run the server
./server.sh

# in the current shell, run the client
./client.sh
```

## Running on nerdserver

to be figured out!
