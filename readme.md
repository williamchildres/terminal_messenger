## Running the Code

This code base packages both client and server binaries.

Server:

```
cargo run --bin server
```

Client

```
cargo run --bin client
```

## Logging

You can control the log level and format by setting environment variables before running your application. For example, you can set `RUST_LOG` to control the log level and format:

```bash
RUST_LOG=info cargo run
```

This sets the log level to `info`, which will display all messages at or above the `info` level. You can also use values like `debug`, `warn`, or `error` for more or less verbose logging.

If you want to redirect the log output to a file instead of displaying it in the console, you can use shell redirection as follows:

```bash
RUST_LOG=info cargo run > logfile.txt 2>&1
```

This redirects both stdout and stderr to a file named "logfile.txt".
