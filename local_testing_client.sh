#!/bin/sh

cargo run --features local --package client-connect --example simple_client wss://127.0.0.1:12345/
