- <https://youtu.be/qXLD2UHq2vk>
  - suggests we use:

     ```shell
     # 10 year expiration
     openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -sha256 -days 3650
     ```

     the passphase I used was "test"


gencert2.sh:
- https://github.com/rustls/tokio-rustls/blob/0184703291746c4875d98456c4ccaca58c25521e/scripts/generate-certificate.sh