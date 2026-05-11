# intersect

a decentralised platform for sharing notes privately. 

content lives on the [veilid](https://veilid.com/) distributed network and is fetched directly in the browser. no backend, no server-side data, no tracking.

notes can be public, private, or password-protected and shareable by link. intersect runs fully in the browser via WebAssembly, connecting to the veilid network directly without any installation or technical knowledge required.

## goals

- **private:** all content is end-to-end encrypted. network traffic is anonymised via veilid's onion routing, similar to Tor.
- **resilient:** no central server to take down. content is redundantly stored across the shared veilid DHT, available as long as nodes exist.
- **accessible:** works in a standard browser, no additional software, plugins, or accounts with third parties needed.
- **open:** the protocol is defined in protobuf, so anyone can build their own client or host their own instance of the webapp. they all use the same underlying network.

## status

active development. core functionality (accounts, notes, access control) is implemented in `intersect-core`. the web ui (`intersect-glasses`) is currently being rewritten. reading works but posting and editing are still in progress. much to come!

a live instance is at [intersect.blog](http://intersect.blog/).
(note: due to soon-to-be resolved limitations in veilid this currently only serves over HTTP, not HTTPS.)

## running locally

1. [install rust](https://www.rust-lang.org/tools/install)
2. install trunk: `cargo install trunk`
3. `cd intersect-glasses && trunk serve --release`
4. open http://localhost:8080/

### docker

```bash
docker-compose up
```

open http://localhost:8080/ once the build finishes.

## structure

| crate | description |
|---|---|
| `intersect-core` | core library, compiles to native and WASM |
| `intersect-cli` | terminal client |
| `intersect-glasses` | web app (leptos + WASM) |
