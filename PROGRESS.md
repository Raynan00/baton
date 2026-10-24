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
- `server/`: Worker skeleton with `/healthz`, Vitest in the Workers pool (1 pass), `tsc` clean.
- CI workflows: `ci-rust`, `ci-server` real; `ci-ios`, `ci-desktop`, `ci-ext` are placeholders that no-op until their projects land; `gitleaks`. **None have run yet** (no CI links to cite).

## In progress
- Nothing mid-flight.

## Next
- Noise IKpsk2 pairing + SAS wiring, Noise_KK LAN link, pairing-link parse/build, SQLite outbox, JSON test vectors.
- Server: Baton-Sig auth, DOs, R2, APNs interface + mock-apns.
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
