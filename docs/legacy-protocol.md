# Protocol 47 transport work

The game still targets its existing modern protocol. `horizon-protocol::legacy18`
adds an experimental wire layer for Minecraft 1.8–1.8.9 (protocol 47). It is **not
connected to the game loop or version-selection UI**, and does not establish
playable 1.8.9 support. No other historical protocols are implemented by this work.

Implemented and covered by byte fixtures and local TCP peers:

- Legacy handshake and username-only login; string UUID profiles.
- Compression negotiation, RSA shared-secret exchange and AES/CFB8 transport.
- A caller-supplied session-service callback. Authentication failure closes the
  connection, without an offline fallback. Tests simulate the session service;
  they do not authenticate a real Minecraft account.
- Direct transition from Login to Play without a modern login acknowledgement.
- Keepalive, chat, join, respawn and relative position codecs, and the legacy
  position/look response. Unknown play packets are explicitly returned as raw
  packets; they are not falsely interpreted as modern packets.
- Outgoing attack/interaction, swing, digging, sprint/sneak, held hotbar slot,
  ground-state and respawn packets, without modern hand or sequence fields.
- Incoming health, held-slot, velocity and block-change updates. Legacy packed
  positions retain X/Y/Z layout rather than the modern X/Z/Y layout.
- A compressed loopback exchange checks action bytes and received block changes;
  this is a transport test, not a Minecraft server or a gameplay test.
- Individual and bulk chunk decoding with strict lengths, little-endian legacy
  states, packed light, sparse sections, biome arrays and unload handling.
  Section views borrow the input packet; they do not allocate block arrays.

The transport reuses Horizon's bounded framing, compression, crypto and socket
implementation. Login checks its negotiation budget between packets and applies
the remaining time to socket reads. Its authentication callback must bound its
own HTTP calls. It is not an end-to-end cancellation mechanism.

## Verify

From the repository root with the pinned Rust toolchain:

```sh
cargo test -p horizon-protocol --lib --test legacy18 --locked
cargo clippy -p horizon-protocol --lib --test legacy18 --locked --no-deps -- -D warnings
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
cargo check -p horizon-protocol --locked --target aarch64-linux-android --target armv7-linux-androideabi --target i686-linux-android --target x86_64-linux-android
```

These checks need no game resources, server account or Android device. They do
not test graphics, real server compatibility or FPS. Cross-target type checking
is not an APK build or a test on a phone.

## Remaining before a playable backend

1. Translate legacy blocks, items, biomes and entity data into the renderer's
   resource registries, with explicit behavior for unmapped states.
2. Implement the remaining play packets and outgoing player actions, inventory
   transactions, containers, entity tracking, resource packs and block updates.
3. Connect the backend to the network/game-thread boundary, account session and
   version selection; preserve modern behavior in its existing backend.
4. Match legacy movement, combat and collision behavior rather than merely
   translating packets. Validate with a reference server and gameplay tests.
5. Build APKs, test representative GPUs and Android versions, and measure frame
   time, memory and sustained thermal behavior in actual multiplayer sessions.

No universal Android support or measured FPS improvement is claimed. Android
version, graphics API, drivers, memory and CPU architecture remain constraints.
The existing Android build floors remain API 28 for 64-bit and API 27 for the
experimental 32-bit builds.

## Build prerequisite correction

The original checkout's lockfile omitted `socks` and its transitive dependencies,
although the workspace already requested that dependency. Resolving the existing
manifests adds those entries to `Cargo.lock`, allowing the targeted commands above
to run with `--locked`. No new dependency was introduced for the legacy module.
