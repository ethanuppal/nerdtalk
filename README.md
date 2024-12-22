# nerdtalk

![CI](https://github.com/ethanuppal/nerdtalk/actions/workflows/ci.yaml/badge.svg)
![Code Style](https://github.com/ethanuppal/nerdtalk/actions/workflows/clippy.yaml/badge.svg)

## Running locally

First, you'll want to generate local certificates

```sh
# mark scripts as executable
chmod u+x ./gen_cert.sh \
    && chmod u+x ./local_tui_client.sh \
    && chmod u+x ./local_server.sh

# generate local testing certificates
./gen_cert.sh

# in a new shell, run the server
./local_server.sh

# in the current shell, run the client
./local_tui_client.sh
```

## Running on nerdserver

to be figured out!
