- <https://youtu.be/qXLD2UHq2vk>
  - suggests we use:

     ```shell
     # 10 year expiration
     openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -sha256 -days 3650
     ```

     the passphase I used was "test"
