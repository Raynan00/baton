# Baton progress

## Plan (Milestone 1)
1. Scaffold monorepo (§16.1), CI, licences, gitleaks. **Done (skeleton).**
2. `baton-core`: proto, HPKE Auth seal/open, SAS, restore-URL, router (**done**); Noise IKpsk2 pairing, Noise_KK LAN link, outbox (SQLite), pairing links, JSON vectors (next).
3. `server/`: device registration + `Baton-Sig`, RendezvousDO, DeviceDO mailbox/WS, R2 blobs, APNs sender behind interface + mock, rate limits.
4. `baton-cli`: `--lan` (mDNS + Noise_KK) and `--relay` modes; E2E tests on both paths.
5. Windows agent (Tauri 2): tray, pair window, keystore, LAN listener, lock/idle, clipboard watcher + concealed filter; MSIX + NSIS in CI.
6. iOS skeleton (XcodeGen; App/NSE/Widget/Share), UniFFI xcframework, `DeviceCapabilities.json` + `TriggerAdvisor`, verified only via `ci-ios`.
7. Spike harnesses + `docs/spikes/S1..S12.md` (priority S1, S3, S4, S6, S10, S11).

## Done
- Monorepo skeleton: Cargo workspace, `config/identity.*`, `proto/baton/v1/baton.proto` (§9.2–9.3), licences (Apache-2.0 root, AGPL-3.0 `server/`), DCO notes, gitleaks, cargo-deny config.
- `baton-core`: protobuf types (prost via protox, no system `protoc`), HPKE Auth-mode seal/open with header-as-AAD (tests: round trip, no plaintext on wire, wrong sender / tampered header rejected), SAS code, restore-URL builder (YouTube `t=`, text fragments, existing `#hash`, tracking-param strip), transport router decision table. `cargo test -p baton-core`: 6 pass.
- `baton-core` (2nd slice): Noise `IKpsk2` pairing handshake with matching SAS on both sides (wrong psk fails), `Noise_KK` LAN link (unknown peer rejected), pairing-link build/parse (params in URL fragment), SQLite outbox (idempotent enqueue, stale drop at 15 min, 1 s→5 min backoff), JSON restore-URL vectors. 13 unit + 1 vector test pass; clippy/fmt clean.
- `baton-cli` relay mode: signed-request client, Noise IKpsk2 pairing through the rendezvous DO, send/receive/ack. `tests/relay_e2e.rs` (set `BATON_RELAY_URL`) passes against a local `wrangler dev` here: matching SAS, E2E `Continue` + `Clip` both ways, unpaired sender rejected, and the Worker log held no URL/title strings. Wired into `ci-server` as `relay-e2e` (not yet run in CI).
- `baton-cli` LAN mode: TCP + `Noise_KK` length-framed link with per-message ack, listener that identifies the initiator by trying paired keys (unknown peers dropped), mDNS advertise/browse with the weekly-rotating hint (`baton-core::discovery`), and `send_with_fallback` (LAN 1.5 s budget → relay). Tests: envelope over LAN, stranger rejected, killed link falls back in < 2 s, mDNS discovery on loopback (opt-in `BATON_TEST_MDNS=1`, enabled in `ci-rust` on Linux).
- `server/` (15 Vitest tests in the Workers pool pass, `tsc` clean):
  - `Baton-Sig` auth (Ed25519 over `METHOD\npath\nts\nhex(sha256(body))`, ±120 s skew); `POST /v1/devices` self-signed registration, device_id = first 16 bytes of sha256(ed25519 pub).
  - `DeviceDO` (SQLite): peer grants, mailbox with TTL alarm, hibernatable WebSocket stream (auth via header on upgrade), ack/delete, spoofed-sender and unpaired-sender rejection, 12 alerts/h limit, stored bytes are opaque ciphertext (asserted).
  - APNs: pure request builder (alert w/ mutable-content + collapse-id + generic text; Live Activity start/update/end), ES256 JWT signer (verified in a test), injectable sender + `MockApns`, 410 clears the token. Real delivery is **unverified** (needs the .p8 and a deployed Worker, §10.5).
  - `RendezvousDO`: 2 parties, 8 frames, 10 min TTL, buffers for a late joiner. `/v1/rendezvous/{rid}` is unauthenticated (rid is a one-time QR capability; phone has no device key before pairing).
  - R2 blobs: quota-checked create, owner PUT, owner/recipient GET with Range, delete = ack. Single-request PUT capped at 32 MiB for now (chunked upload later).
- CI workflows: `ci-rust`, `ci-server` real; `ci-ios`, `ci-desktop`, `ci-ext` are placeholders that no-op until their projects land; `gitleaks`. **None have run yet** (no CI links to cite).

## In progress
- Nothing mid-flight.

## Next
- `baton-core`: socket framing for the LAN link, mDNS advert/discovery, HPKE/pairing JSON vectors (HPKE needs deterministic-RNG hook).
- Server: R2 lifecycle backstop test, `MODE=gateway` `/v1/push`, per-device 120 msg/min and 24 LA/h limits, `Ack{REJECTED,UNREACHABLE}` on 410, standalone `mock-apns` HTTP server for `wrangler dev` e2e.
- `baton-cli`: a `clap` front-end (pair/send/receive, `--lan`/`--relay`); LAN pairing path (Noise IKpsk2 over the LAN socket) and an e2e that pairs over LAN with no server.
- Remaining items from the plan above.

## Blocked: needs Raynan
- Apple Developer account: APNs auth key (.p8) → `wrangler secret put APNS_KEY_P8`, plus `APNS_KEY_ID`, `APNS_TEAM_ID`, `APNS_TOPIC`; App Groups, Associated Domains, App Clip, signing, TestFlight.
- Cloudflare account + `wrangler login`; `CLOUDFLARE_API_TOKEN` as a CI secret to enable `deploy-server`; R2 bucket `batonsync-blobs` with a 1-day lifecycle rule.
- Domain `batonsync.app` (availability unchecked) for the relay custom domain, AASA and landing page.
- Partner Center: reserve the Baton app name for the MSIX; signing for direct installers.
- A real iPhone + Mac/Xcode for spikes S1–S10; a real Windows PC for S11.

## Spike results
- None run yet.

## Decisions (ADRs)
- Server dev deps pinned to `@cloudflare/vitest-pool-workers ~0.9` + `vitest ~3.2` (newer 0.23 is deprecated/renamed and needs vitest 4); `.npmrc legacy-peer-deps=true` works around a wrangler/workers-types peer clash. Small enough to skip a formal ADR; revisit when migrating to `@cloudflare/vitest-plugin`.
