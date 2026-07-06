# dexdey

Dexdey is an experimental minecraft proxy server written in Rust with support for velocity plugins.

## Running

```bash
cargo run
```

## Features

### Proxy

- [x] Forwarding with `forwarding.secret`
- [ ] Configuration
- [ ] Multiple backend servers
- [ ] Backend server selection
- [ ] Skin support
- [x] Encryption
- [x] Compression

### Protocol

- [x] Java Edition 26.2 (776)

### Plugins

- [x] Plugin loading
- [x] Event: ProxyInitializeEvent
