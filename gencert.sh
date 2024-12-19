#! /bin/sh

cd cert
rm ./*
openssl req -x509 -noenc -subj '/CN=example.com' -newkey rsa -keyout root.key -out root.crt
openssl req -noenc -newkey rsa -keyout client.key -out client.csr -subj '/CN=example.com' -addext subjectAltName=DNS:localhost
openssl x509 -req -in client.csr -CA root.crt -CAkey root.key -days 365 -out client.crt -copy_extensions copy
cd ..