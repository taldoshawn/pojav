# Protocol 47 transport work

The default game still targets the modern protocol. The development backend
now connects protocol 47 to the game via `--protocol-47`, and Android's version
chooser selects that connection path. This is not yet a validated playable
release: APK rendering and interaction tests remain pending. Other historical
protocols are not implemented.

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

1. Complete missing entity metadata, resource packs and inventory
   transaction reconciliation. Object spawns now translate dropped items,
   arrows, other legacy projectiles and vehicles; potion effect updates also
   reach the game. These additions still need visual gameplay validation. Blocks, items, biomes, player lists, equipment,
   mob metadata and scoreboards now have translation paths into the game.
2. Match legacy movement, combat and collision behavior rather than merely
   translating packets. Validate with a reference server and gameplay tests.
3. Test the integrated APK's rendering and touch controls, then representative
   GPUs and Android versions. Measure frame time, memory and sustained thermal
   behavior in actual multiplayer sessions.

No universal Android support or measured FPS improvement is claimed. Android
version, graphics API, drivers, memory and CPU architecture remain constraints.
The existing Android build floors remain API 28 for 64-bit and API 27 for the
experimental 32-bit builds.

## Build prerequisite correction

The original checkout's lockfile omitted `socks` and its transitive dependencies,
although the workspace already requested that dependency. Resolving the existing
manifests adds those entries to `Cargo.lock`, allowing the targeted commands above
to run with `--locked`. No new dependency was introduced for the legacy module.

## Integration work in progress

The legacy connection uses the real account session callback, fixed dimension
registries, and a separate outgoing action translator. Chunks enter the streaming
renderer after old block IDs are upgraded through the existing world data fixers.
Partial columns retain omitted sections; light and biome colors use legacy data.
Health, teleports, chat, basic player entities and inventory updates reach the game.
Unknown packet families and incomplete prediction/transactions remain limitations.

A native protocol probe joined a disposable Cuberite server, received its spawn
and four columns, and checked 262,144 block states. The extended probe also
received server confirmation of block placement and breaking, and sent movement. This proves transport against
an independent server; it does not prove Android rendering, PvP or server-wide
compatibility. `tools/legacy_test_server.py` provisions the pinned server fixture
and records server-side touch-test movement/block events for the Android harness.

Protocol field behavior was compared against Cuberite's `Protocol18x.cpp` at
[72aa50f710200832f97a0022dc70c5cd531b6aea](https://github.com/cuberite/cuberite/tree/72aa50f710200832f97a0022dc70c5cd531b6aea),
Apache-2.0. No Cuberite implementation source is incorporated into the game.
The test server is downloaded to disposable storage, not bundled in the APK.

## Android regression evidence

The protocol-47 APK from build 9 rendered the Cuberite world and displayed the
native FPS counter. Server logs confirmed touch movement and block placement.
The initial combined action check did not confirm breaking: repeated placement
on the software-rendered emulator displaced the player from the target. The
harness now resets the fixture between independent placement and break checks.
This evidence is not yet a passing gameplay result or a phone FPS measurement.

The isolated two-client wire probe also received server-confirmed attack damage.
The Android harness has a separate touch-attack check against that second client;
full combat parity is not inferred from a single attack.
