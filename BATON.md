# Baton: build doc (kickoff prompt + spec + research)

> **How to use this file.** It's the only document you need. Commit it to the repo root as `BATON.md`, then paste **Part 1** into your cloud Claude Code session (or just tell the agent "Read BATON.md and follow Part 1").
>
> - **Part 1: Kickoff prompt.** Instructions for the coding agent.
> - **Part 2: Spec.** Scope, architecture, protocol, crypto, the Cloudflare server, UX copy, milestones and acceptance criteria. Sections are numbered **§0–§22**.
> - **Appendix A: Research.** Why the design looks like this: iOS limits, prior art, App Review. Sections are numbered **A1–A9**.
>
> Owner: Raynan Wuyep · Version 0.2 · 23 Oct 2026

---

# Part 1: Kickoff prompt for Claude Code

You're the lead engineer on **Baton**, an open-source "Continuity for everyone" app. It's Apple-style Handoff, Universal Clipboard and file drop across **iPhone, Windows and Linux** (Android and Mac come later).

## Goal
Get as close to seamless as the platforms allow, with very easy setup. **Works anywhere, LAN when possible.**
- **PC → iPhone:** when I lock my PC or go idle, the iPhone Lock Screen already shows "Continue: *Title* · 62%" (an end-to-end encrypted push plus a push-to-start Live Activity). One tap opens Safari at the **same paragraph** (text-fragment URL) or YouTube at the **same timestamp**.
- **iPhone → PC:** one gesture (Action button, Control Center control or Back Tap, picked **per device**) or the Share sheet.
- Clipboard and files both ways. Install and pair in **under 90 s**. No accounts.
- **Transport:** go **direct over the local network** whenever both devices can reach each other (fastest and most private). Use the **Cloudflare server** whenever that's more useful: devices on different networks, waking the iPhone through APNs, queueing for an offline device, and large files through temporary encrypted R2 blobs. Both paths carry the same end-to-end encrypted envelopes.

## Source of truth
1. **This file (`BATON.md`) is authoritative.** Part 2 covers scope, architecture, protocol, crypto, the server API, UX copy, milestones and acceptance criteria. Read Part 2 fully before writing code.
2. **Appendix A explains *why*** (iOS limits, prior art, App Review). Read A1, A3, A5 and A9 at minimum.
3. If you need to deviate from the spec, write an ADR in `docs/adr/NNNN-title.md` and note it in `PROGRESS.md`. Don't silently diverge. Don't edit `BATON.md` itself without an ADR that says why.

## Hard constraints
- **Never commit secrets.** That covers Apple `.p8` keys, signing certificates, App Store Connect API keys, Cloudflare API tokens, Partner Center credentials, FCM credentials and `.env` files. Add `gitleaks` to CI and keep only `.env.example` / `.dev.vars.example` files. Read secrets from the environment, `wrangler secret`, or CI secrets.
- **Don't fake anything that needs an Apple Developer account, a real device, a Cloudflare account or secrets.** That includes APNs delivery, Live Activity push-to-start, App Groups and keychain sharing on device, Associated Domains, the App Clip, TestFlight, code signing, `wrangler deploy` and Microsoft Store submission. Build them behind clear interfaces with mocks for CI, then **flag them** in `PROGRESS.md` under `## Blocked: needs Raynan` with the exact step he must take (e.g. "Create an APNs auth key (.p8) in Certificates, IDs & Profiles, then run `wrangler secret put APNS_KEY_P8` and add `APNS_KEY_ID`, `APNS_TEAM_ID` as Worker secrets").
- **You're on a Linux machine. You can't build, sign or run iOS here.** Write the Swift, the XcodeGen `project.yml` and the XCTests, but verify iOS only through the **`ci-ios` GitHub Actions workflow on `macos-latest`** (Xcode 26+, simulator, no signing). Keep iOS changes in small PRs so CI failures are easy to pin down. Windows builds and tests run on `windows-latest` in CI. You *can* build and test Rust, the Cloudflare Worker (`wrangler dev`, Vitest with `@cloudflare/vitest-pool-workers`), the Linux desktop app and the browser extension locally.
- **Local `workerd` / `wrangler dev` can't speak HTTP/2, and APNs requires HTTP/2.** Production Workers reach APNs fine. Keep the APNs sender behind an interface, test it against a mock APNs server locally and in CI, and only claim real delivery after a deployed test (§10.5).
- **No iOS background modes, no silent-push sync, no keep-alive tricks** (App Review 2.5.4). Be honest in code and copy: iOS can't keep a LAN listener alive in the background, so the LAN path only works while the Baton app (or one of its extensions) is running. Push content is end-to-end encrypted with generic fallback text (4.5.4).
- **No accounts or sign-in in the MVP.** Identity is per-device keys, created at pairing. Don't add email, OAuth or Sign in with Apple.
- Use the **placeholder identity** `batonsync` (domain `batonsync.app`, bundle prefix `app.batonsync`) from `config/identity.*` only. Never hard-code names elsewhere.
- **iOS deployment target 17.2.** Gate iOS 18+ features (Controls) with `#available`.
- Device-aware trigger logic must come from the **data table** (`DeviceCapabilities.json`, §8.2), not from model-number heuristics. iPhone Duo (2026) has **no** Action button, and iPhone 16e/17e have an Action button but **no** Dynamic Island.

## How to work
- **Small PRs or commits**, each building and testing on its own, with conventional commit messages (`feat(core): …`, `fix(relay): …`). Open a PR per checklist item where practical.
- **Tests come with the code:** unit and property tests in `baton-core`; Worker tests with Vitest + `@cloudflare/vitest-pool-workers` (Durable Objects, R2 and a mock APNs); end-to-end tests where `baton-cli` fake devices talk over **both** the LAN path and the relay path; Playwright tests for the extension; table-driven XCTests for the iOS capability and trigger logic. Every row of §8.2 and §8.3 gets a test case.
- **Keep `PROGRESS.md` current** after every meaningful step, with these sections: `## Done`, `## In progress`, `## Next`, `## Blocked: needs Raynan`, `## Spike results`, `## Decisions (ADRs)`.
- Prefer boring, well-maintained crates and libraries (§4). Run `cargo deny` and `npm audit`/`license-checker` for licences and advisories. Clients and core are Apache-2.0 and the server is AGPL-3.0 by default (§21), so check dependency licences against that.
- When something in the spec is ambiguous, pick the simplest option that keeps the acceptance criteria true, record it in `PROGRESS.md`, and keep going. Only stop for things that need Raynan (accounts, secrets, money, naming, licence).

## Order of work (Milestone 1 first)
1. **Scaffold the monorepo** exactly as in §16.1: Cargo workspace, `proto/`, `config/identity`, `apps/desktop` (Tauri 2 + Svelte), `apps/ios` (XcodeGen), `extensions/browser` (WXT), `server/` (Cloudflare Worker, TypeScript), `web/`, GitHub Actions (`ci-rust`, `ci-server`, `ci-desktop`, `ci-ext`, `ci-ios`), `gitleaks`, licences, README, `PROGRESS.md`.
2. **`baton-core`:** protobuf types (§9.2–9.3), HPKE Auth-mode seal/open, Noise `IKpsk2` pairing with a 6-digit SAS, the `Noise_KK` LAN link, pairing-link parse/build, the transport router (LAN first, relay fallback, §3.2), the outbox (SQLite), the restore-URL builder (text fragments, YouTube `t=`), and JSON test vectors.
3. **`server/` (Cloudflare Worker) + `baton-cli`:** device registration with signed requests, the rendezvous Durable Object, peer grants, the per-device mailbox Durable Object (WebSocket stream + queue + TTL alarms), R2 blobs with lifecycle expiry, the APNs sender (alert + Live Activity start/update/end) behind an interface with a mock, rate limits, `wrangler.toml` with dev and prod environments. `baton-cli` gets a LAN mode (mDNS + direct link) and a relay mode.
4. **Windows desktop agent:** tray, pair window (QR + SAS confirm), keystore, LAN listener + mDNS advertisement, lock/idle/sleep detector, clipboard watcher with concealed-format filtering, outbox through the transport router. Package as **MSIX for the Microsoft Store** and as a direct-download installer (§4, §16.2). Linux implementations behind the same traits come later.
5. **iOS skeleton:** targets App, NSE, Widget (Live Activity + Control + Lock Screen widget), Share. A UniFFI xcframework build script. Universal-link + in-app QR scanner pairing. Bonjour browsing while in the foreground. `DeviceCapabilities.json` + `TriggerAdvisor` (§8) with tests. NSE decrypt path against fixture payloads.
6. **Spike harnesses early** (§14). Build the minimal code and a `docs/spikes/S<n>.md` template for each of S1–S12, so Raynan can run them on a real iPhone and a real Windows PC as soon as the accounts are ready. Prioritise **S1 (Live Activity push-to-start), S3 (background Copy action), S4 (clipboard read from intents), S6 (App Shortcuts in Action button / Back Tap pickers), S10 (LAN from the app and extensions) and S11 (MSIX + native messaging)**. Each spike doc needs a go/no-go criterion and a fallback, and the product code must keep working with the fallback.
7. Then follow §17 for M2 → M4.

## Definition of done: Milestone 1
All of the following must be true and shown in `PROGRESS.md`, with links to CI runs:
- [ ] `cargo test --workspace` passes on Linux and Windows CI. `clippy -D warnings` and `fmt` are clean. `ci-server` (typecheck, lint, Vitest in the Workers pool) is green.
- [ ] Two `baton-cli` devices pair **over the LAN** (mDNS discovery, no server involved) and, separately, **through a locally running Worker** (`wrangler dev`). Both show the **same SAS** and exchange E2E `Continue` and `Clip` messages. A test asserts that Durable Object storage, R2 objects and logs contain **no plaintext** (URL and title strings are absent).
- [ ] The transport router picks LAN when the peer is reachable and falls back to the relay within 2 s when it isn't (test with the LAN link killed mid-session).
- [ ] An unpaired sender is rejected by the Worker. Expired-TTL messages and blobs are gone. The APNs mock receives correctly shaped `alert` (mutable-content, collapse-id) and `liveactivity` start/update/end requests.
- [ ] The Windows build from `ci-desktop` produces an **MSIX** (unsigned or test-signed in CI) and a per-user installer. Either one installs, shows the pairing QR, and pairs with `baton-cli` (a scripted test or a documented manual run with screenshots).
- [ ] The Windows clipboard watcher **skips** clips marked `ExcludeClipboardContentFromMonitorProcessing` (automated test).
- [ ] The iOS project generates with XcodeGen, **builds for the simulator and passes unit tests on `macos-latest`** (capability table, trigger advisor, FFI vectors, link parsing).
- [ ] Spike harness code and docs exist for S1–S12.
- [ ] `PROGRESS.md` lists every item blocked on Raynan (Apple account, APNs key, Cloudflare account and `wrangler` login, domain/AASA, Partner Center app reservation, iOS signing).

Start by reading Part 2 and Appendix A. Then post a short plan (≤ 15 bullets) in `PROGRESS.md` and begin step 1.

---

# Part 2: Spec (v0.2)

> **Continuity for everyone.** Lock your PC and pick up your iPhone. What you were reading or watching is already on your Lock Screen. One tap opens it at the same paragraph or timestamp. Clipboard and files move the same way, direct over Wi-Fi when you're home and through a small encrypted relay when you're not. It works across Windows, Linux and iPhone, then Android and Mac.

- **Status:** draft for build kickoff · **Owner:** Raynan Wuyep · **Date:** 23 Oct 2026
- **Source of truth.** This file is the source of truth. Appendix A holds the research behind it. Where they disagree, Part 2 wins. Log any change as an ADR in `docs/adr/`.
- **Tags:** **[SPIKE-n]** = unverified assumption with a go/no-go test (§14). **[unverified]** = not confirmed in a primary source. **[decide]** = Raynan's call (§21).
- **Naming:** the product is **Baton** (decided). All `baton.*` .com, `.app`, `.dev` and `.to` domains and `github.com/baton` are taken, so technical identifiers use **`batonsync`**: domain `batonsync.app` (availability **not yet checked**), bundle prefix `app.batonsync`, repo `batonsync/baton`, Worker `batonsync-relay`. Keep these in **one config file per platform** (`config/identity.*`) so a change is one line.

---

## 0. TL;DR of key decisions

| # | Decision | Why |
|---|---|---|
| D1 | **The desktop decides when a handoff is happening** (lock, idle, sleep). The server then sends an **E2E-encrypted APNs push** plus a **push-to-start Live Activity** ("Continue on iPhone"). | iOS has no unlock event and no background daemon. This is as zero-tap as iOS allows. |
| D2 | **Restore by URL alone:** a text-fragment URL (`#:~:text=`) and YouTube `t=`. A Safari Web Extension adds exact scroll and video time as an optional extra. | It works in stock Safari with no extension installed. |
| D3 | **One Rust core** (protocol, crypto, pairing, transport router, outbox) shared by the desktop, the iOS app and every iOS extension (via UniFFI), `baton-cli`, and later Android. | One implementation of the security-critical code. |
| D4 | **Stateless per-message encryption (HPKE Auth mode)** after a **Noise IKpsk2** QR pairing with a SAS confirm. | The NSE, Share extension and app can each decrypt on their own, and the same envelope works over LAN or the relay. |
| D5 | **Works anywhere, LAN when possible.** Two first-class paths carry identical E2E envelopes: **direct LAN** (mDNS/Bonjour discovery, Noise-encrypted link) whenever both ends are reachable, and the **Cloudflare server** (Workers + Durable Objects + R2) for remote relay, queueing for offline devices, large-file blobs and APNs wake-ups. | LAN is fastest and most private. The server makes it work across networks and is the only way to wake a closed iPhone. |
| D6 | **The server is a small Cloudflare Worker** that only ever sees ciphertext, device IDs and push tokens. It holds the Apple `.p8` key as a Worker secret. Everything stored is short-lived (≤ 24 h). Self-hosters can deploy the same Worker to their own Cloudflare account and push through the official **push gateway**. | No servers to run, near-zero cost at MVP scale, no egress fees on R2, global edge. |
| D7 | **Device-aware iOS trigger.** Pick the best trigger in this order: Action button › Control Center control › Back Tap › Lock Screen widget › Share sheet. The choice comes from the model identifier plus the iOS version. | The Action button is missing on many iPhones, including the new **iPhone Duo**. |
| D8 | **Minimum iOS 17.2.** | This is the hard floor for push-to-start Live Activities. Everything newer is gated with `#available` (§8.5). |
| D9 | **No iOS background modes in the MVP** (no BLE, audio or location). No silent-push sync. | App Review 2.5.4, battery, simplicity. |
| D10 | **No accounts or sign-in in the MVP.** Identity is per-device keys, and pairing is the only "signup". Account-based features (multi-device sync groups, off-network history, recovery) are a later phase (§19). | Fastest onboarding. Avoids the login and Sign in with Apple requirements. |
| D11 | **Windows ships through the Microsoft Store (MSIX) and a direct installer.** Raynan already has a Windows developer account and a published Store app (localpdf), so Store signing and distribution are covered. | Store-signed MSIX avoids SmartScreen warnings at no extra cost, and the Store brings search traffic. |

---
## 1. Goals, non-goals, success metrics

**Goals**
1. **Seamless:** desktop → iPhone handoff appears with **0 taps** and opens in **1 tap**. iPhone → desktop takes **1 gesture** (Action button, Control or Back Tap) or 2 taps (Share sheet).
2. **Easy:** installing on both devices plus pairing takes **under 90 s at the median**. Every optional extra is one screen with a deep link.
3. **Trustworthy:** E2E encryption, a blind server, LAN direct whenever possible, no content in logs or telemetry, and open source.

**Success metrics (MVP, measured on the real-device matrix in §16.4)**

| Metric | Target |
|---|---|
| Install + pair time (median, new user, desktop already downloaded) | < 90 s |
| Lock PC → card visible on iPhone Lock Screen (p50 / p95) | < 3 s / < 10 s |
| Tap card → page open at the right paragraph | < 2 s, and the paragraph is in the viewport ≥ 90% of the time on test articles |
| YouTube resume accuracy | ± 2 s |
| Copy on PC → "Copy" action available on phone | < 5 s p95 |
| iPhone share → PC toast | < 5 s p95 |
| Desktop agent idle CPU / RAM (tray only, window closed) | < 1% / < 80 MB |
| iOS battery impact | No background modes, NSE work < 1 s per push |

**Non-goals (MVP):** see §19.

---

## 2. Personas & core flows

- **P1 "Windows + iPhone" (primary).** Uses Chrome or Edge, reads and watches on the laptop, then moves to the phone (and, in Lagos, often away from the desk during NEPA power cuts).
- **P2 "Linux dev + iPhone".** Uses Firefox or Chrome on GNOME or KDE and cares about self-hosting.

| Flow | Trigger | Result |
|---|---|---|
| **F1 Continue → iPhone** | PC locks, goes idle ≥ 60 s on a continuable item, sleeps, or the user presses the hotkey | Live Activity and/or a notification: "Continue: *Title* · 62%". Tap opens Safari or YouTube at the saved position |
| **F2 Continue → PC** | iPhone Share sheet "Send to PC", the Action button, Control or Back Tap, or (opt.) the Safari extension's automatic capture | PC toast: "Continue from iPhone: *Title*". Click opens it in the browser, and the extension restores scroll and time |
| **F3 Clip → iPhone** | Copy on the PC. Sending follows the chosen mode (§7.4) | Notification: "Copied on PC · 24 chars" with a **Copy** action, plus clip history in the app |
| **F4 Clip → PC** | The iPhone trigger runs "Send clipboard to PC" | The PC clipboard is set and a quiet toast confirms it |
| **F5 File → iPhone** | PC "Send to iPhone" (tray drag-drop, Explorer *Send To*, file picker) | Notification "PC sent *photo.jpg* (3.2 MB)" → tap saves to Files or Photos |
| **F6 File → PC** | iPhone Share sheet → Baton | The file lands in `~/Downloads/Baton` with a toast |
| **F7 Withdraw** | PC unlocks or input resumes within 2 min | The Live Activity ends and the notification is removed, so nothing stays stale |

---
## 3. System architecture

### 3.1 Components
```
┌──────────── Desktop (Windows → Linux → macOS) ─────────────┐
│ Browser ext (MV3: Chrome/Edge/Firefox)                     │
│   tab URL, title, scroll fraction, text fragment, video t  │
│        │ native messaging (JSON, stdio)                    │
│ baton-nmhost (tiny binary) ──local IPC (pipe/UDS)──┐       │
│ Baton Agent (Tauri 2 tray app, Rust)  ◄────────────┘       │
│   handoff detector · clipboard watcher · file sender       │
│   LAN listener + mDNS advert (_batonsync._tcp)             │
│   baton-core (protocol, HPKE, Noise, router, outbox, store)│
└──────┬──────────────────────────────────────────┬──────────┘
       │ ① LAN direct (preferred when reachable)  │ ② HTTPS + WebSocket
       │   TCP + Noise_KK, same E2E envelopes     │
       │                                ┌─────────▼──────────────────────────┐
       │                                │ Cloudflare server (batonsync-relay)│
       │                                │ Worker: auth, routing, rate limits │
       │                                │ DeviceDO: mailbox · WS · tokens    │
       │                                │ RendezvousDO: pairing frames       │
       │                                │ R2: encrypted temp blobs (≤ 24 h)  │
       │                                │ APNs sender (.p8 secret) · FCM later│
       │                                └─────────┬──────────────────────────┘
       │                                          │ APNs: alert (mutable) ·
       │                                          │ liveactivity start/update/end
┌──────▼──────────────────────────────────────────▼──────────┐
│ iOS app (SwiftUI) + baton-core xcframework (UniFFI)        │
│  Bonjour browse + LAN link (foreground only)               │
│  NSE (decrypt/render) · Widget ext (Live Activity, Lock    │
│  Screen widget, Control) · Share ext · App Intents/App     │
│  Shortcuts · (opt) Safari Web Ext · (later) Keyboard ext   │
│  · (spike) App Clip for instant pairing                    │
└────────────────────────────────────────────────────────────┘
   (Phase 2: BLE GATT desktop-peripheral ↔ iPhone-central, ANCS)
```

### 3.2 Transport router ("works anywhere, LAN when possible")
Every message is an HPKE-sealed `Envelope` (§9.2). The router in `baton-core` only decides *how* it travels, so the security story is identical on both paths.

| Situation | Path | Why |
|---|---|---|
| Peer has a live LAN link (connected or reachable in the last 10 s) | **LAN direct** | Fastest, nothing leaves the network |
| Peer not reachable on the LAN (other network, mobile data, AP isolation, Local Network permission denied) | **Relay** (`POST /v1/messages`, delivered over the peer's WebSocket or queued) | Works anywhere |
| Recipient is an iPhone whose app isn't in the foreground (the usual case at handoff time) | **Relay + APNs push** (alert and/or Live Activity) | The only way to reach a closed iOS app. Any LAN send would fail, because iOS can't listen in the background |
| Recipient offline (PC asleep, phone off) | **Relay mailbox** (queued ciphertext, TTL ≤ 24 h) | Delivered on reconnect |
| File ≤ 8 MB, or any size while a LAN link is up | LAN stream if available, otherwise an R2 blob | |
| File > 8 MB and no LAN link | **R2 blob** (chunked, encrypted, ≤ 24 h) + a `FileOffer` push. When the phone opens the app and a LAN link appears, it **upgrades** and fetches straight from the PC instead | Big files still work remotely, and go fast when you're home |
| Duplicate delivery (sent on both paths during a switch) | Dedup on `msg_id` | |

Rules:
- **Send-once semantics:** try LAN with a 1.5 s connect/ack budget, then fall back to the relay. Handoff messages to an iPhone skip the LAN attempt unless the phone app is known to be foreground (it holds an open LAN link or a relay WebSocket and reported `foreground=true`).
- **Honest iOS behaviour:** the iPhone's LAN link exists only while the Baton app, Share extension or an intent is running. iOS suspends it seconds after backgrounding and the router immediately treats the phone as "relay only" again.
- **Discovery:** the desktop advertises `_batonsync._tcp` over mDNS with a random instance name and a TXT record holding `h = first 8 bytes of BLAKE2s(device_id ‖ weekly salt)`, so passers-by can't track a stable ID. Paired devices recognise each other's `h`. The QR code also carries the desktop's current LAN addresses as a hint.
- **LAN link security:** TCP, then `Noise_KK_25519_ChaChaPoly_BLAKE2s` using the static keys learned at pairing. Unknown peers are dropped before any payload is read. Envelopes inside are still HPKE-sealed, so the code path is the same as the relay's.
- **No relay configured** (a "LAN-only" toggle in Settings, off by default) is supported for privacy purists. The app then explains that Lock Screen handoff to a closed iPhone won't work.
- The `Transport` trait keeps room for later paths (BLE, Wi-Fi Aware, EU accessory APIs).

### 3.3 Data flows (summary)

| Flow | Typical path |
|---|---|
| F1 Continue → iPhone (PC locks) | Relay → APNs alert + Live Activity. Tap opens the restore URL |
| F2 Continue → PC | LAN if the PC is reachable from the Share extension/intent within 1.5 s, otherwise relay → PC WebSocket (or queue if the PC sleeps) |
| F3 Clip → iPhone | With the handoff push (relay), or LAN if the Baton app is open |
| F4 Clip → PC | LAN first, relay fallback |
| F5 File → iPhone | LAN if the app is open. Otherwise R2 blob + push, with a LAN upgrade on open |
| F6 File → PC | Share extension: LAN if reachable, otherwise R2 blob via a background `URLSession` |
| Pairing | Noise over LAN if the phone finds the PC, otherwise over the rendezvous Durable Object |

---

## 4. Tech stack per component

| Component | Stack | Notes |
|---|---|---|
| `baton-core` | Rust 2021 stable. `snow` (Noise), `hpke` (RFC 9180), `ed25519-dalek`, `x25519-dalek`, `chacha20poly1305`, `blake2`, `prost` (protobuf), `rusqlite` (outbox/store), `tokio`, `reqwest` + `tokio-tungstenite`, `mdns-sd` (desktop discovery) | No platform code. `#![forbid(unsafe_code)]` except in the FFI crate |
| `baton-ffi` | UniFFI → Swift (xcframework: `aarch64-apple-ios`, `aarch64-apple-ios-sim`, `x86_64-apple-ios`) and Kotlin later | A narrow API surface (§9.6). iOS uses `NWBrowser`/`NWConnection` for Bonjour and sockets and passes bytes into the core |
| `baton-platform` | Rust traits `Clipboard`, `IdleLock`, `Notifier`, `KeyStore`, with Windows impls (`windows` crate: `AddClipboardFormatListener`, `WTSRegisterSessionNotification`, `GetLastInputInfo`, DPAPI/Credential Manager via `keyring`) and Linux impls (`zbus`: logind `Lock`/`Unlock`, `org.freedesktop.ScreenSaver`; Wayland `ext-idle-notify-v1`; clipboard via `ext-data-control-v1`/`wlr-data-control` or X11; Secret Service via `keyring`) | Linux Wayland/GNOME is a known pain point → [SPIKE-8] |
| Desktop app | **Tauri 2** (Rust) with a Svelte 5 + Vite + TypeScript UI. Tray-first, and the window is destroyed when closed to free the WebView | **Windows: MSIX for the Microsoft Store** (first-class; the Store signs it) **plus a direct download** (MSIX sideload or NSIS per-user installer; sign direct builds once a certificate or Azure Trusted Signing is set up). Linux: AppImage + .deb. Later: winget, Scoop, Flathub, AUR |
| `baton-nmhost` | A Rust binary that speaks the Chrome/Firefox native-messaging stdio protocol and bridges to the agent over a named pipe or Unix socket | Host name `app.batonsync.host`. Registered per-user (HKCU on Windows, `~/.config/google-chrome/NativeMessagingHosts`, `~/.mozilla/native-messaging-hosts` on Linux). MSIX registry virtualization may hide the HKCU entry from browsers → [SPIKE-11] |
| Browser ext | TypeScript, MV3, **WXT** (one codebase for Chrome, Edge, Firefox, and later Safari). Uses GoogleChromeLabs `text-fragments-polyfill` fragment-generation utils | Permissions: `nativeMessaging`, `tabs`, `scripting`, `storage`, `<all_urls>` host permission (justify in the store listing). Edge Add-ons uses the same Partner Center account |
| Server (`server/`) | **Cloudflare Worker in TypeScript** (Wrangler 4). **Durable Objects** with SQLite storage: `DeviceDO` (keys, push tokens, peer grants, mailbox, hibernatable WebSocket, TTL alarms) and `RendezvousDO` (pairing frames). **R2** bucket for encrypted blobs with a 1-day lifecycle rule. APNs over `fetch` with an ES256 JWT signed via WebCrypto from the `APNS_KEY_P8` secret. Optional **Workers Rate Limiting** binding. FCM HTTP v1 later. KV only if a real need appears | Tests: Vitest + `@cloudflare/vitest-pool-workers`, plus a mock APNs. `ENV=dev|prod`, `MODE=full|gateway`. Same code self-hosts on any Cloudflare account |
| iOS | Swift 6, SwiftUI, **XcodeGen** (`apps/ios/project.yml`, so no `.pbxproj` merge pain). Targets: App, NSE, WidgetExtension (Live Activity, Lock Screen widget, Control), ShareExtension, (opt) SafariWebExtension, (spike) AppClip, (later) Keyboard. App Group `group.app.batonsync`. Shared keychain access group. `NSBonjourServices = [_batonsync._tcp]`, `NSLocalNetworkUsageDescription` | Built with **Xcode 26+ / iOS 26 SDK** (App Store requirement since 28 Apr 2026). Deployment target **17.2** |
| Android (post-MVP) | Kotlin, foreground service (`connectedDevice`), share target, QS tile, FCM, NSD for LAN | Same core through UniFFI |
| Web | Static site (Astro or plain HTML) on **Cloudflare Pages** at the same domain: landing page, `/.well-known/apple-app-site-association`, `/p` pairing fallback page, privacy policy, `device-capabilities.json` | Same Cloudflare account as the Worker |

---
## 5. Onboarding (target < 90 s)

### 5.1 Happy path timeline

| t (s) | Desktop | iPhone |
|---|---|---|
| 0 | Install from the **Microsoft Store** (or the direct download: MSIX or `BatonSetup.exe`, per-user, no admin, no reboot). The agent starts in the tray and opens the **Pair** window with a large QR code | — |
| 10 | The window shows the QR plus "Scan with your iPhone camera" and a secondary **"Add browser extension"** button (opens the store page; the extension connects to the native host automatically on install) | Open **Camera** and point it at the QR |
| 15 | — | **Path A [SPIKE-7]:** the App Clip card appears → **Pair** (no install). **Path B:** a universal link. If the app is installed it opens straight to pairing. If not, the `/p` page opens with an App Store button (Smart App Banner) → **Get** |
| 15–45 | — | (Path B) App Store install, about 20–30 s on a decent connection |
| 45 | — | Open Baton. It goes straight to the in-app scanner ("Scan the code on your PC again"). The desktop QR stays valid for 10 min and rotates afterwards. Pairing runs straight over Wi-Fi if the phone can see the PC (iOS shows the **Local Network** prompt here, after a primer), otherwise through the relay. Either way it takes about a second |
| 50 | Desktop shows: "**Raynan's iPhone** wants to pair. Code **482 913**" → **[Pair]** | Phone shows the same code: "Check that your PC shows **482 913**" |
| 55 | Paired ✓ | **Notifications** primer → system prompt (the only permission asked now). Live Activities need no prompt; they're on by default and can be turned off in Settings |
| 60–85 | Tray toast: "Baton is ready. Lock your PC to try it." | **One device-aware setup card** (§8): e.g. "Press-and-hold your Action button to send to your PC → **Set up (3 taps)**" or **Skip** |
| 90 | Done | Done |

**Deferred pairing.** iOS has no reliable deferred deep link through the App Store. Use the App Clip (Path A) or **re-scan** (Path B). Never use the pasteboard, which triggers the paste prompt, and never use fingerprinting.

### 5.2 Just-in-time permissions

| Permission | When asked | Copy (primer before the system prompt) |
|---|---|---|
| Notifications (iOS) | Right after pairing succeeds (needed for the core) | "Let your PC tap you on the shoulder. Baton shows what you were doing on your PC, end-to-end encrypted. No ads, no marketing." |
| Camera (iOS, in-app scanner only) | When the in-app scanner opens | "To scan the pairing code on your PC." |
| Photos (add-only) | The first time the user saves a received image to Photos | "Save photos your PC sends you." (`NSPhotoLibraryAddUsageDescription`) |
| Paste | The first time "Send clipboard" runs. Then offer **"Paste from Other Apps → Allow"** via the app's Settings page (iOS 16.1+ per-app setting) [SPIKE-4] | "iOS asks before any app reads your clipboard. Set Baton to **Allow** so it doesn't ask every time." |
| Local Network (iOS) | At the first pairing scan, just before the phone looks for the PC. Denying it is fine: Baton falls back to the relay | "Find your PC on this Wi-Fi, so things move faster and stay on your network." |
| Desktop notifications / startup | Default **on**, with a toggle in Settings | — |
| Browser extension `<all_urls>` | At extension install (browser UI) | Store listing: "Baton reads the page you're on only to save your place. Nothing leaves your devices unencrypted." |

---

## 6. Handoff moment detection (desktop)

**Continuable item** = the focused browser tab where:
- the article has scroll fraction 0.05–0.95 and page height ≥ 2 viewports; **or**
- a `<video>` has `currentTime ≥ 30 s` and `duration − currentTime ≥ 60 s`; **or**
- the user pressed the hotkey (**Ctrl+Alt+B**, configurable), which sends regardless.

**Events → actions**

| Event | Source (Win / Linux) | Action |
|---|---|---|
| Screen lock | `WM_WTSSESSION_CHANGE`/`WTS_SESSION_LOCK` / logind `Lock` signal, `org.freedesktop.ScreenSaver.ActiveChanged` | Debounce 3 s → send `Continue(reason=LOCK)` if continuable |
| Idle ≥ 60 s (configurable) | `GetLastInputInfo` poll every 5 s only while a continuable item exists / `ext-idle-notify-v1` or ScreenSaver idle | Send `Continue(reason=IDLE)`. Skip if a media item is still *playing* (the user is watching) |
| Sleep / lid close | `WM_POWERBROADCAST PBT_APMSUSPEND` / logind `PrepareForSleep` | Send synchronously, best effort, 2 s budget |
| Unlock / input resumes ≤ 2 min after a send | Same sources | Send `Withdraw(item_id)` → the server ends the Live Activity and the NSE removes the delivered notification (via `apns-collapse-id` replace + app-side removal) |
| Hotkey | Global shortcut (Tauri plugin) | Send `Continue(reason=MANUAL)` immediately |

**Anti-spam rules:** keep one *replaceable* card per desktop (stable `collapse_key = H(desktop_id)`). Cooldown: the same URL is not re-alerted within 10 min. Cap alerts at ≤ 12 per hour per phone (the server enforces this too).

---

## 7. Feature specs

### 7.1 Capture & restore (browser)

Extension content script → `TabState` (debounced 1.5 s on scroll/visibility/media events, sent only to the local native host):
- `url` (canonical, with tracking params stripped: `utm_*`, `fbclid`, `gclid`), `title`, `site_name` (og:site_name), `favicon_url`
- `scroll_fraction` = scrollY / (scrollHeight − innerHeight)
- `text_fragment`: generated for the first fully visible block element of ≥ 40 chars near the top third of the viewport. Uses the polyfill generator with prefix/suffix for uniqueness. Validate by finding exactly one match. If none, leave it empty
- `media`: for the most prominent `<video>` (largest visible area), `{current_time, duration, playing}`. YouTube: also `video_id`

**Restore URL builder (core, shared):**
1. YouTube (`youtube.com/watch`, `youtu.be`, `/shorts` excluded) → `https://youtu.be/<id>?t=<floor(sec)>`. The universal link opens the YouTube app if it's installed, otherwise Safari.
2. Article with a fragment → `url#:~:text=<fragment>` (supported in Safari 16.1+, Chrome and Firefox 131+). If the URL already has a `#hash`, append it as `#hash:~:text=…`.
3. Otherwise, the plain URL. The Safari extension (opt.) or desktop extension restores `scroll_fraction` and `media.current_time` when it detects a matching tab load within 60 s ("pending restore" keyed by normalized URL).

### 7.2 F1 desktop → iPhone delivery

- **Alert push** (always, unless the policy in [SPIKE-1/2] decides LA-only): `mutable-content: 1`, generic fallback text, `apns-collapse-id = collapse_key`, `thread-id = "continue"`, interruption level `active`, category `CONTINUE` with actions **Open** (foreground) and **Not now**. The NSE decrypts and sets title/body/thumbnail-free content.
- **Live Activity** "Continue on iPhone": push-to-start on the first send, then `update` for later items (one activity per desktop), and `end` on Withdraw or after 4 h (`stale-date` 1 h). Layout: app glyph · title (1 line) · site · progress bar (scroll or video) · "from Raynan-PC · 2 min ago". Tapping it deep-links `batonsync://open?item=<id>`, and the app opens the restore URL. [SPIKE-9] checks whether a `Link` straight to the https URL avoids the app flash.
- **Content privacy in the Live Activity:** the ActivityKit payload doesn't pass through the NSE. **Default:** the content-state carries only `enc` (an HPKE blob) plus generic fields (`"Continue from your PC"`, progress). The widget extension *tries* to decrypt `enc` with the shared-keychain key at render time [SPIKE-2]. If that's impossible, the LA shows generic text plus progress, and the real title appears only in the NSE-rendered notification.

### 7.3 F2 iPhone → desktop

- **Share extension** (URL, text, images, files): encrypt in the extension → send over the LAN link if the PC answers within 1.5 s, otherwise upload via a background `URLSession` to R2 (files) or a direct POST to the server (small) → dismiss with "Sent to Raynan-PC ✓". Pin it as "Send to PC" (the user can favourite it).
- **App Intent `SendToPCIntent`** (App Shortcut phrase: "Send to PC with Baton"). Behaviour: if there's clipboard text or a URL → send it as `Clip`/`Continue`. Run with `openAppWhenRun = false`. Reading the clipboard depends on [SPIKE-4].
- **Safari Web Extension** (optional, M4): captures the active tab on `visibilitychange: hidden` → native handler → App Group outbox → network [SPIKE-5].
- **Desktop:** toast "Continue from iPhone: *Title*" → click → open in the default browser and register a pending restore. Option "Open automatically when I'm at my PC" (off by default).

### 7.4 Clipboard

| Direction | Modes | Default |
|---|---|---|
| PC → iPhone | **(a) When I leave my PC:** a text clip copied ≤ 2 min before the handoff event is attached to the Continue push as a **Copy** action. **(b) Hotkey** Ctrl+Alt+C. **(c) Every copy** (notification per copy, rate-limited) | (a) + (b) |
| iPhone → PC | Trigger (§8) → `SendToPCIntent`, Share sheet on selected text, the in-app `UIPasteControl` | — |

**Safety (all mandatory):**
- Skip clips with concealed markers. Windows: the formats `ExcludeClipboardContentFromMonitorProcessing`, `CanIncludeInClipboardHistory = 0`, `CanUploadToCloudClipboard = 0`. Linux: `x-kde-passwordManagerHint = secret`. macOS later: `org.nspasteboard.ConcealedType`/`TransientType`. Verify each password manager in the test matrix (KeePassXC, 1Password, Bitwarden).
- Process denylist (password managers), plus a **Pause clipboard sync** toggle in the tray.
- Size cap: text ≤ 64 KB (bigger goes as a file). Images ≤ 10 MB as a blob. Clips expire after 24 h on every device.
- Notifications **redact by default**: "Copied on PC · 24 characters". Preview is opt-in.
- The iOS **Copy** action writes `UIPasteboard.general` with `expirationDate` 10 min, `localOnly = true` (doesn't feed Apple's Universal Clipboard). Background feasibility is [SPIKE-3].

### 7.5 Files

- Encrypt with a random per-file 32-byte key using chunked AEAD (XChaCha20-Poly1305, 1 MiB chunks, STREAM construction: 24-byte nonce = 19-byte random prefix ‖ u32 BE chunk counter ‖ 1-byte last-chunk flag; implemented once in the core). The key and hash go inside the HPKE-sealed `FileOffer`. The same ciphertext format is used on both paths.
- **LAN path (preferred when reachable):** the sender streams the encrypted chunks over the Noise LAN link. Resumable by chunk index.
- **Relay path:** `POST /v1/blobs` → a short-lived upload URL (Worker-proxied PUT to R2, or an R2 presigned URL) → `FileOffer` message (+ push to iPhone). Default limits on the hosted server are configurable: 100 MB per file, 1 GB/day per device, 24 h TTL, deleted on ack, R2 lifecycle rule as a backstop.
- **LAN upgrade:** if a `FileOffer` arrives via the relay but the receiver then has a LAN link to the sender (e.g. the phone app opens at home), it fetches from the sender directly and acks, and the blob is deleted.
- iOS receive: tap the notification → LAN fetch if available, else background `URLSession` download from R2 → decrypt → Files (`Baton` folder in On My iPhone via `UIFileSharingEnabled` + `LSSupportsOpeningDocumentsInPlace`) or Photos (add-only). Batches > 50 MB on iOS 26+: `BGContinuedProcessingTask` with progress.
- Desktop receive: `~/Downloads/Baton/`, plus a toast with Open and Show in folder.
- Desktop send entry points: drag onto the tray window, the tray menu "Send file…", and Explorer integration. **NSIS build:** a **Send To** shortcut (`shell:sendto`). **MSIX build:** the package manifest's File Explorer context-menu extension (this also gives the Windows 11 modern context menu). Details in [SPIKE-11].

---
## 8. iOS device-aware trigger setup

### 8.1 Detection at onboarding (and on every app launch, cheap)

```swift
var sys = utsname(); uname(&sys)
let machine = withUnsafeBytes(of: &sys.machine) { String(decoding: $0.prefix { $0 != 0 }, as: UTF8.self) }
// Simulator: ProcessInfo.processInfo.environment["SIMULATOR_MODEL_IDENTIFIER"]
let os = ProcessInfo.processInfo.operatingSystemVersion
```
- Look up `machine` in **`DeviceCapabilities.json`**, bundled in the app and refreshable from `https://batonsync.app/device-capabilities.json` (signed with the release Ed25519 key, cached). New iPhones then work without an app update.
- Fields: `{ id, name, actionButton: bool, dynamicIsland: bool, homeButton: bool, cameraControl: bool, backTap: bool }`.
- **Unknown identifier** (a future model): show one question with an illustration, "Does your iPhone have an Action button (above the volume buttons)?" Yes/No. Don't infer from the number, because **iPhone Duo (2026) has no Action button**.
- Runtime capability checks (don't rely on the table for these): `ActivityAuthorizationInfo().areActivitiesEnabled`, notification settings, `#available(iOS 18, *)` for Controls.
- Can't detect: what the Action button or Back Tap is currently set to (no API). So ask: "Use your Action button for Baton? (It currently does whatever you set it to.)" → Use it / Keep mine → fall to the next option.

### 8.2 Device capability table (bundled seed, verified 23 Oct 2026)

Supported devices are those that can run iOS ≥ 17.2: iPhone XS/XR and later, and SE 2nd gen and later. iOS 26/27 support iPhone 11 and later plus SE 2/3. XS/XR top out at iOS 18.

| Model | Identifier(s) | Action btn | Dynamic Island | Notes |
|---|---|---|---|---|
| iPhone XS / XS Max / XR | iPhone11,2 · 11,4 / 11,6 · 11,8 | ✕ | ✕ | Max iOS 18 |
| iPhone SE (2nd / 3rd gen) | iPhone12,8 · 14,6 | ✕ | ✕ | Home button, Touch ID |
| iPhone 11 / 11 Pro / 11 Pro Max | iPhone12,1 · 12,3 · 12,5 | ✕ | ✕ | |
| iPhone 12 mini / 12 / 12 Pro / 12 Pro Max | iPhone13,1 · 13,2 · 13,3 · 13,4 | ✕ | ✕ | |
| iPhone 13 mini / 13 / 13 Pro / 13 Pro Max | iPhone14,4 · 14,5 · 14,2 · 14,3 | ✕ | ✕ | |
| iPhone 14 / 14 Plus | iPhone14,7 · 14,8 | ✕ | ✕ | |
| iPhone 14 Pro / 14 Pro Max | iPhone15,2 · 15,3 | ✕ | ✓ | |
| iPhone 15 / 15 Plus | iPhone15,4 · 15,5 | ✕ | ✓ | Ring/Silent switch |
| iPhone 15 Pro / 15 Pro Max | iPhone16,1 · 16,2 | ✓ | ✓ | The only Action-button phones that can be on iOS 17 |
| iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max | iPhone17,3 · 17,4 · 17,1 · 17,2 | ✓ | ✓ | Camera Control |
| **iPhone 16e** | iPhone17,5 | **✓** | **✕** (notch) | No Camera Control |
| iPhone 17 / 17 Pro / 17 Pro Max / Air | iPhone18,3 · 18,1 · 18,2 · 18,4 | ✓ | ✓ | |
| **iPhone 17e** (Mar 2026) | iPhone18,5 | **✓** | **✕** | |
| iPhone 18 Pro / 18 Pro Max (2026) | iPhone19,2 · 19,3 / 19,7 | ✓ | ✓ (smaller) | Up to 3 LAs in the DI |
| **iPhone Duo** (foldable, 2026) | iPhone19,4 | **✕** | ✓ (outer display) | Touch ID side button, Camera Control |

Back Tap is available on every supported device (it needs iPhone 8+ and iOS 14+). Sources: Apple "Identify your iPhone model" (support.apple.com/108044), Apple Back Tap (support.apple.com/111772), MacRumors and 9to5Mac on the iPhone Duo. Identifiers come from third-party lists (DeviceAtlas, IPSW Core, AppleDB), because Apple doesn't publish them. **Confirm the 2026 identifiers (17e, 18 Pro, Duo) on real devices or in Xcode's device list.**

### 8.3 Trigger choice (iPhone → PC "Send to PC")

Priority: **Action button › Control Center control › Back Tap › Lock Screen widget › Share sheet** (the Share sheet always works, so it's never "set up").

| Device / iOS | Primary (shown as the one setup card) | Secondary ("More ways" sheet) | Setup steps shown |
|---|---|---|---|
| Action button + iOS 18+ | **Action button → Baton Control** | Control Center, Back Tap | "Settings › Action Button › swipe to **Controls** › Choose › **Baton: Send to PC**" (≈ 4 taps). Button: open Settings (`UIApplication.openSettingsURLString` opens *Baton's* page; there is no public deep link to Action Button settings, so show an animated guide) |
| Action button + iOS 17.2–17.x (15 Pro only) | **Action button → Shortcut → Baton "Send to PC"** (App Shortcut, no building) [SPIKE-6] | Back Tap, Lock Screen widget | "Settings › Action Button › **Shortcut** › Baton › Send to PC" |
| No Action button + iOS 18+ | **Control Center control** | Back Tap, Lock Screen control (replace the flashlight/camera button) | "Swipe down from top-right › hold › **Add a Control** › search *Baton*" (≈ 4 taps), with an animated guide |
| No Action button + iOS 17.2–17.x | **Back Tap** | Lock Screen widget (interactive, iOS 17+) | "**Add Shortcut**" (one tap, via an iCloud shortcut link *if* App Shortcuts aren't listed in Back Tap [SPIKE-6]) → "Settings › Accessibility › Touch › Back Tap › Double Tap › Send to PC" |
| Any, user says "Keep my Action button" | Next row down | — | — |
| Any | **Share sheet** is always mentioned once: "You can also tap Share › Baton anywhere." | — | — |

**Implementation**
- `SendToPCIntent: AppIntent` plus `AppShortcutsProvider` with phrases. Shortcuts then appear automatically in the Shortcuts app and the Action button picker, so nobody builds a Shortcut by hand. Use `ShortcutsLink` (iOS 16+) on the "More ways" sheet.
- `SendToPCControl: ControlWidget` (`ControlWidgetButton(action: SendToPCIntent())`, `@available(iOS 18, *)`), with `.displayName("Send to PC")` for the Action-button hint.
- An interactive Lock Screen accessory widget `Button(intent: SendToPCIntent())`.
- Fallback shortcut: one published iCloud shortcut ("Baton: Send Clipboard": *Get Clipboard* → *Send to PC* intent) for Back Tap, *only* if [SPIKE-4]/[SPIKE-6] require it.
- Record `trigger_setup = {kind, confirmed_at}` locally. Confirm it worked when the first intent invocation arrives with `source = actionButton|control|backTap` (infer from the invocation where possible, otherwise "any"). Then show "✓ Action button works!" once.
- **Camera Control isn't used.** It launches camera experiences, not arbitrary intents [unverified for iOS 27; recheck].

### 8.4 Graceful degradation by iOS version

| Capability | 17.2–17.x | 18.x | 26.x | 27.x |
|---|---|---|---|---|
| Alert push + NSE decrypt | ✓ | ✓ | ✓ | ✓ |
| Live Activity push-to-start / update / end | ✓ | ✓ | ✓ | ✓ |
| Dynamic Island presentation | DI models only | same | same | same (18 Pro: up to 3) |
| Control Center / Lock Screen controls, Action button → Control | ✕ (use a Shortcut) | ✓ | ✓ | ✓ |
| Interactive Lock Screen / Home widgets | ✓ | ✓ | ✓ | ✓ |
| App Shortcuts (no manual building) | ✓ | ✓ | ✓ | ✓ |
| `BGContinuedProcessingTask` (big file batches) | ✕ → foreground download + background URLSession | ✕ | ✓ | ✓ |
| `LongRunningIntent` | ✕ | ✕ | ✕ | [unverified], not used in the MVP |
| Notification-received automations | ✕ | ✕ | ✕ | [unverified whether they run without confirmation], not used |
| Live Activities turned off by the user | Alert-only fallback, with a hint in Settings | same | same | same |
| Notifications denied | Persistent in-app banner: "Baton can't reach you". The PC shows "iPhone notifications off" | same | same | same |

### 8.5 Minimum iOS: **17.2**
- It's the floor for **push-to-start** Live Activities, the core "Handoff icon".
- It keeps iPhone XS/XR (stuck on iOS 18) and anyone still on 17.x. Apple's App Store data (7 Jun 2026): iOS 26 on 79% of all iPhones, iOS 18 on 14%, earlier versions on 7%.
- Supporting 17.x costs little: just the `#available(iOS 18)` paths for controls. **Revisit at M4.** If TestFlight and analytics show < 1% of users on 17.x, raise it to 18.0 and delete the Shortcut-only Action-button path.

---

## 9. Protocol

### 9.1 Principles
- Protobuf (`proto/baton/v1/*.proto`) is the schema source of truth. `prost` is used in Rust, and all other platforms go through the core, so there's no Swift/Kotlin protobuf.
- **Versioning:** `Envelope.v` is the wire major version (1). The inner payload uses proto3 evolution rules (only add fields, never reuse tags). Devices exchange `protocol_min/max` and a `caps` bitset in `DeviceInfo`. A receiver ignores unknown payload kinds with `Ack{status=UNSUPPORTED}`.
- Native messaging (extension ↔ host) is **JSON** (a browser requirement), versioned with `"v":1`, and plaintext over local stdio only.

### 9.2 Outer envelope (visible to the server; identical on the LAN link)
```proto
message Envelope {
  uint32 v = 1;                 // 1
  bytes  msg_id = 2;            // 16 random bytes; idempotency + replay key
  bytes  sender = 3;            // 16-byte device_id
  bytes  recipient = 4;         // 16-byte device_id
  uint64 sent_at_ms = 5;
  uint32 ttl_s = 6;             // relay drops after TTL (max 86400)
  PushHint push = 7;            // NONE | ALERT | LA_START | LA_UPDATE | LA_END
  bytes  collapse_key = 8;      // opaque 16 bytes (H(desktop_id)), used for apns-collapse-id
  bytes  hpke_enc = 9;          // HPKE encapsulated key
  bytes  ciphertext = 10;       // HPKE Auth-mode seal of Payload; AAD = fields 1–8 serialized
}
```

### 9.3 Inner payload (E2E)
```proto
message Payload { oneof kind {
  Continue continue = 1;  Withdraw withdraw = 2;  Clip clip = 3;
  FileOffer file_offer = 4;  Ack ack = 5;  DeviceInfo device_info = 6;
  KeyRotation key_rotation = 7;  Unpair unpair = 8; } }

message Continue { string item_id = 1; Reason reason = 2; /* LOCK IDLE SLEEP MANUAL SHARE EXTENSION */
  string url = 3; string restore_url = 4; string title = 5; string site_name = 6;
  float scroll_fraction = 7; string text_fragment = 8;
  Media media = 9; uint64 captured_at_ms = 10; string source_app = 11; /* "chrome","edge","safari" */
  ClipRef attached_clip = 12; }
message Media { double current_time_s = 1; double duration_s = 2; bool playing = 3; string youtube_id = 4; }
message Withdraw { string item_id = 1; }
message Clip { string clip_id = 1; oneof body { string text = 2; BlobRef image = 3; }
  uint32 char_count = 4; uint64 expires_at_ms = 5; }
message ClipRef { string clip_id = 1; }   // clip sent alongside, same push
message FileOffer { string file_id = 1; string name = 2; uint64 size = 3; string mime = 4;
  bytes sha256 = 5; bytes file_key = 6; bytes nonce_prefix = 7; uint32 chunk_size = 8; BlobRef blob = 9; }
message BlobRef { string blob_id = 1; uint64 size = 2; }
message Ack { bytes msg_id = 1; Status status = 2; } // DELIVERED OPENED REJECTED UNSUPPORTED
message DeviceInfo { string name = 1; Platform platform = 2; string model_id = 3; string os_version = 4;
  uint32 protocol_min = 5; uint32 protocol_max = 6; uint64 caps = 7; string app_version = 8; }
message KeyRotation { bytes new_x25519_pub = 1; uint64 valid_from_ms = 2; bytes sig_ed25519 = 3; }
message Unpair { }
```
APNs payload size is 4 KB. If the encoded envelope is > 3 KB, the push carries only `msg_id` and the NSE fetches it from the server.

### 9.4 Crypto design

| Item | Choice |
|---|---|
| Device identity | Ed25519 signing key (relay auth, signed records) + X25519 static key (Noise/HPKE). Generated on device and never exported |
| Key storage | iOS Keychain, shared access group across app, NSE, widget, share and Safari extensions, `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly` (the NSE must decrypt while locked). Windows: DPAPI / Credential Manager. Linux: Secret Service, with a fallback to a 0600 file encrypted with a key held by the Secret Service, or a warning if it's missing |
| Pairing | `Noise_IKpsk2_25519_ChaChaPoly_BLAKE2s` (`snow`). The phone is the initiator and knows the desktop's static key from the QR. psk = 32-byte one-time QR secret. Frames go over a direct LAN TCP connection when the phone can reach the PC (the QR carries LAN address hints and mDNS confirms them), otherwise over the rendezvous Durable Object WebSocket (§10). The transcript exchanges `DeviceInfo` and Ed25519 public keys |
| SAS | 6 digits = `u32(BLAKE2s("baton-sas-v1" ‖ handshake_hash)[0..4]) mod 10^6`, shown on both devices and confirmed on the **desktop** (one click). This defends against a photographed or relayed QR |
| Messages | **HPKE Auth mode** (RFC 9180): DHKEM(X25519, HKDF-SHA256) / HKDF-SHA256 / ChaCha20-Poly1305, `info = "baton/v1/msg"`, AAD = envelope header. Stateless, works out of order, multi-process safe |
| LAN link | TCP + `Noise_KK_25519_ChaChaPoly_BLAKE2s` with both static keys from pairing. Carries the same HPKE-sealed envelopes as the relay. Unknown keys are dropped before any payload is read (§3.2) |
| Replay | `msg_id` seen-set (7 days, App Group store) + `sent_at` window ±24 h |
| Forward secrecy | No per-message FS (a trade-off for statelessness). Rotate the X25519 key weekly via `KeyRotation` (signed with Ed25519) and delete old private keys 48 h after rotation. MLS or a double ratchet is a post-MVP ADR |
| Files | Per-file key + STREAM chunked XChaCha20-Poly1305 (§7.5) |
| Server auth | Every HTTP request is signed: `Authorization: Baton-Sig id=<device_id>,ts=<unix>,sig=<ed25519(method‖path‖ts‖sha256(body))>`, ±120 s skew. WebSocket: a signed first frame |
| Authorization | The server only accepts messages from A to B if B registered A as a peer (a `PeerGrant` signed by B during pairing). This stops push spam to tokens |
| Unpair | Either side sends `Unpair` + `DELETE /v1/peers/{id}`. Local keys for that peer are wiped |

### 9.5 Test vectors
Ship `crates/baton-core/tests/vectors/*.json` (pairing transcript, SAS, HPKE seal/open, STREAM chunks). Swift tests run the same vectors through the FFI.

### 9.6 Core FFI surface (UniFFI)
`Core::open(app_group_path)`, `identity()`, `parse_pairing_link(url)`, `pair_start(link) -> PairingSession{sas}`, `pair_confirm()`, `peers()`, `unpair(id)`, `seal(payload, peer) -> Envelope`, `open(envelope_bytes) -> Payload`, `outbox_enqueue/flush()`, `restore_url(continue)`, `register_push(apns_token, la_start_token, env)`, `fetch(msg_id)`, `lan_peer_seen(addrs, txt)`, `lan_frame_in(bytes) -> Vec<bytes>`, `route(peer) -> Lan|Relay|RelayWithPush`. Keep it synchronous where possible (the NSE has a short time budget). Async calls go through UniFFI async support.

---

## 10. Server API (v1): the Cloudflare Worker

Base `https://relay.batonsync.app` (a custom domain on the Worker) [decide domain]. JSON over HTTPS except `envelope` fields (base64url of protobuf bytes). All endpoints except `/healthz` need `Baton-Sig` (§9.4).

### 10.1 Cloudflare layout

| Piece | Job | Storage / lifetime |
|---|---|---|
| Worker (`fetch` handler) | Verifies `Baton-Sig`, rate-limits, routes to the right Durable Object, issues blob URLs, signs the APNs JWT | Stateless. APNs JWT cached in memory (refresh every 50 min) |
| `DeviceDO` (one per `device_id`) | Public keys, push tokens, peer grants, the **mailbox queue**, the device's hibernatable WebSocket (`/v1/stream`), per-device counters | DO SQLite. Mailbox rows purged on ack or by alarm at TTL (≤ 24 h). Tokens and grants until unpair or delete |
| `RendezvousDO` (one per pairing `rid`) | Relays opaque Noise frames between two parties when they can't pair over LAN | In memory + alarm. 10 min TTL, max 2 participants, max 8 frames |
| R2 bucket `batonsync-blobs` | Encrypted file chunks for relay-path transfers | Lifecycle rule deletes after 1 day. Deleted on ack |
| Secrets | `APNS_KEY_P8`, `APNS_KEY_ID`, `APNS_TEAM_ID`, `APNS_TOPIC`, (later) `FCM_SERVICE_ACCOUNT`, `GATEWAY_ALLOWLIST` | `wrangler secret put`, never in the repo |

### 10.2 Endpoints

| Method & path | Body → Response | Notes |
|---|---|---|
| `POST /v1/devices` | `{ed25519_pub, x25519_pub, platform}` → `{device_id}` | Self-signed with the new key (proof of possession). No account |
| `PUT /v1/devices/me/push` | `{apns_token?, apns_env: sandbox\|production, la_push_to_start_token?, fcm_token?}` | Called on every token change |
| `PUT /v1/devices/me/presence` | `{foreground: bool}` | Optional hint for the router (§3.2). Expires after 60 s |
| `DELETE /v1/devices/me` | — | Wipes the mailbox, tokens and grants |
| `WS /v1/rendezvous/{rid}` | Opaque Noise frames between two parties | `rid` from the QR |
| `POST /v1/peers` | `{peer_id, grant_sig}` | Mutual grant after pairing |
| `DELETE /v1/peers/{peer_id}` | — | |
| `POST /v1/messages` | `{envelopes:[b64]}` → `{accepted:[msg_id], rejected:[{msg_id, reason}]}` | Delivers over the recipient's WebSocket if connected, else queues it, then sends a push per the `push` hint |
| `GET /v1/messages?since=<cursor>` | → `{envelopes:[…], cursor}` | Mailbox drain |
| `GET /v1/messages/{msg_id}` | → envelope | Used by the NSE for oversize payloads |
| `DELETE /v1/messages/{msg_id}` | — | Ack = delete |
| `WS /v1/stream` | Server → client envelopes. Client → server acks | Desktops stay connected (hibernatable WebSocket, so idle connections cost almost nothing). Phones connect while the app is foreground |
| `POST /v1/blobs` | `{size, chunks}` → `{blob_id, upload_urls\|put_url, expires_at}` | Quota-checked |
| `GET /v1/blobs/{id}` / `DELETE` | Ciphertext bytes (range requests supported) | Range reads straight from R2 |
| `POST /v1/push` *(gateway mode only)* | `{token, kind, apns_payload_without_content}` from an allow-listed self-hosted server key | Lets self-hosters reach the App Store build, which only Raynan's `.p8` key can push to |
| `GET /healthz` | — | Workers analytics only. No per-device labels or IPs |

### 10.3 APNs mapping
`ALERT` → `apns-push-type: alert`, `apns-priority: 10`, `apns-collapse-id`. `LA_START` → push type `liveactivity`, topic `<bundle>.push-type.liveactivity`, `event: start` with `attributes-type: ContinueAttributes`, `content-state`, `alert` (generic). `LA_UPDATE` / `LA_END` → `event: update|end` with `dismissal-date` (now for Withdraw). Handle `410 Unregistered` by marking the token dead and notifying the sender (`Ack{REJECTED, reason=UNREACHABLE}`). Use `api.sandbox.push.apple.com` for `apns_env=sandbox`.

### 10.4 Rate limits and quotas (per device)
120 msgs/min, 12 alerts/h per recipient, 24 LA pushes/h, blob quota per §7.5. Over the limit → `429` + `Retry-After`.

### 10.5 Cloudflare specifics and gotchas
- **HTTP/2 to APNs:** production Workers reach APNs over HTTP/2 through Cloudflare's proxy. Local `workerd` / `wrangler dev` has no HTTP/2, so local runs and CI use a mock APNs (or a local HTTP/2 upgrading proxy for manual tests). Mark real delivery as verified only after a deployed test against `api.sandbox.push.apple.com` ([workerd #4841](https://github.com/cloudflare/workerd/issues/4841)).
- **Cost:** at MVP scale this should fit the Workers Free plan, or Workers Paid at about $5/month once Durable Object or request limits are hit. R2 has no egress fees. Confirm current pricing before launch [unverified].
- **Self-hosting:** `wrangler deploy` to your own account with `MODE=full` and `GATEWAY_URL` set. Push requests are then forwarded (content-free, already encrypted) to the official gateway.
- Keep everything in `server/`, with `wrangler.toml` environments `dev` (sandbox APNs) and `prod`.

---

## 11. Data retention, privacy, security

| Data | Where | Retention |
|---|---|---|
| LAN traffic | Never leaves the local network | — |
| Message ciphertext | `DeviceDO` mailbox | Until acked or TTL (≤ 24 h) |
| File ciphertext | R2 | Until acked or 24 h (lifecycle rule as a backstop) |
| Push tokens, public keys, peer grants | `DeviceDO` | Until unpair or device delete. Tokens are purged on APNs 410 |
| IP addresses | Not stored by Baton (Cloudflare edge logs per Cloudflare's policy; Baton logs drop IPs and bodies) | — |
| Clip history | Each device (App Group / local DB) | 24 h default, with options for 1 h, 7 days or off |
| Continue history | Each device | 7 days, last 50 |
| Rendezvous frames | `RendezvousDO` memory | 10 min |

- **The server learns:** which device IDs talk, when, payload sizes, push tokens, and IPs (transiently, at the edge). **It never learns:** URLs, titles, clip contents, file names or contents. **On the LAN path it learns nothing at all.**
- **Push fallback text is generic** ("New item from your PC") so that a failed NSE leaks nothing (App Review 4.5.4).
- Privacy policy and App Store privacy label: "Data Not Collected" when telemetry is off. With opt-in telemetry: "Diagnostics, not linked to you".
- Threat model notes (`docs/threat-model.md`): a lost phone means unpairing from the desktop. A malicious server can drop or delay messages and see metadata, but can't read or forge content. A malicious device on the LAN sees an mDNS advert with a rotating hash and can't complete `Noise_KK` without a paired key. QR shoulder-surfing is caught by the SAS plus one-time psk + 10 min expiry.

---
## 12. Error, offline & edge-case handling

| Situation | Behaviour |
|---|---|
| LAN peer unreachable mid-send | Router falls back to the relay within 2 s. Dedup on `msg_id` if both arrive |
| Local Network permission denied (iOS) | Relay only, silently. Settings shows "Direct Wi-Fi off: using the relay" with a button to fix it |
| Wi-Fi with client isolation (hotels, some offices) | mDNS finds nothing → relay. No user-facing error |
| Server unreachable (desktop) | LAN still works if the peer is reachable. Otherwise the outbox (SQLite) retries with exponential backoff (1 s → 5 min, jitter). Tray shows "Offline: will send when connected". Handoff items older than 15 min are dropped (stale) |
| Server unreachable (iOS share) | LAN if the PC is reachable, else a background URLSession retries. The share sheet still says "Queued ✓" |
| NSE fails or times out | The generic notification shows. Tapping it opens the app, which fetches and renders |
| APNs token invalid | The desktop shows "Your iPhone isn't reachable. Open Baton on it once." |
| Notifications / LAs disabled | §8.4 fallbacks. The desktop shows the status per paired phone |
| QR expired / already used | Phone: "This code expired. Your PC is showing a new one." The desktop auto-rotates it |
| SAS mismatch / desktop declines | Abort, wipe the session, and log a local security event |
| Clock skew > 120 s | The server returns `401 skew`. The client retries with a server-time hint once and warns if it persists |
| Duplicate delivery (LAN + relay, or WS + push) | Dedup on `msg_id` |
| App Group SQLite lock while suspended | Don't hold DB locks across suspension (avoids the `0xdead10cc` termination). Short transactions, and close before `beginBackgroundTask` expires. The NSE uses its own read-only connection plus a file-based inbox |
| Desktop asleep while the phone sends | Queued in the mailbox. Delivered on wake. The toast says "(sent 2 h ago)" |
| Multiple PCs | Each PC has its own Live Activity (≤ 2 active). The phone "Send to PC" defaults to the last-active PC, and a long-press chooses another |

---

## 13. Battery & performance

- **iOS:** no background modes, no silent pushes, no polling. The app does work only in the foreground (including Bonjour browsing and the LAN link, which stop on backgrounding), in the NSE (< 1 s; keep the core's NSE path small because NSE memory is limited), in intents and extensions, and in system-run background URLSessions. Live Activities are updated only on real handoff events (≤ 24/h hard cap, typically < 10/day).
- **Desktop:** event-driven, with idle polling only while a continuable item exists. One relay WebSocket with 60 s pings, plus an idle LAN listener. Destroy the WebView when the window closes. The extension sends to the native host only on debounced changes.
- **Server:** hibernatable WebSockets mean idle desktops cost close to nothing. Load-test 10k connected desktops on a deployed `dev` environment at M4.

---

## 14. Prototype spikes (run early: Week 1–2, on real devices)

| # | Question | Test | Go if | Fallback |
|---|---|---|---|---|
| **S1** | Is Live Activity push-to-start reliable at handoff frequency? | 3 phones (DI and non-DI), 10 pushes/day for 3 days, start/update/end, sent from the deployed Worker | ≥ 95% appear in ≤ 10 s, no silent throttling, and `end` removes it | Alert push only (collapse-id replaceable), plus a Lock Screen widget showing the "last item" |
| **S2** | Can the widget extension decrypt `enc` in the LA content-state at render time (shared keychain)? | A render test on locked and unlocked devices | The title renders while locked (after first unlock) | Generic LA text + progress. The real title is only in the NSE notification |
| **S3** | Can a **non-foreground** notification action write `UIPasteboard`? | A `Copy` action without `.foreground` on iOS 17/18/26/27 | The pasteboard is set without opening the app | A `.foreground` Copy action: the app opens, copies, and shows "Copied ✓ · swipe back" |
| **S4** | Can `SendToPCIntent` read the clipboard without a prompt once "Paste from Other Apps = Allow" is set, when run from the Action button, a Control or Back Tap while another app is foreground? | The matrix on 17/18/26 | No prompt and the text is read | (a) The iCloud shortcut does *Get Clipboard* (Shortcuts' own paste setting) → passes text into the intent. (b) The intent opens the app with a `UIPasteControl` button |
| **S5** | Is iOS Safari Web Extension capture reliable (native messaging → App Group → network)? | 50 page-hides across sites | ≥ 80% reach the PC in ≤ 5 s | Share sheet only. The extension is used only for *restoring* scroll and time |
| **S6** | Do App Shortcuts appear in the Action button and Back Tap pickers without the user building a shortcut? | A fresh install on 17.x and 18+ | They're listed in both | A one-tap "Add Shortcut" iCloud link (prebuilt) |
| **S7** | Can an App Clip (≤ 15 MB for QR/physical invocation) do pairing including the Rust core, and hand keys to the full app via App Group + keychain? | Build size + a flow test | ≤ 15 MB, and the handoff to the full app works | Path B (universal link → App Store → re-scan) |
| **S8** | Linux Wayland clipboard and idle on GNOME and KDE | Ubuntu 24.04 GNOME, Fedora KDE | Watcher works on KDE + X11 | GNOME: clipboard only via hotkey (read on demand while focused) and a documented limitation |
| **S9** | Can a Live Activity tap open the https restore URL directly (no app flash)? | A `Link` / `widgetURL` with https | Opens Safari/YouTube directly | Deep link into the app → `UIApplication.open(restoreURL)` |
| **S10** | **LAN from iOS:** does Bonjour discovery + a `Noise_KK` link work from the app *and* from the Share extension / an App Intent (Local Network permission is per app, shared by its extensions [unverified]), and how fast is it? | Home Wi-Fi, a mesh network, a phone hotspot and a hotel-style isolated network; app foreground, Share extension, Action button intent | App finds the PC in ≤ 2 s on normal Wi-Fi. Extensions/intents connect within the 1.5 s budget ≥ 90% of the time | App-only LAN; extensions and intents always use the relay |
| **S11** | **Windows MSIX:** inside an MSIX package, do the native-messaging host registration (HKCU, possibly virtualized), the startup task, the clipboard listener, lock detection, the LAN listener (firewall prompt) and the Explorer context menu all work? Does the Store accept the needed capabilities? | Package with the Tauri build, sideload on Windows 10 22H2 and 11 24H2, then a Store submission to private/hidden flight | All features work, and certification passes | Register the host via `desktop6:RegistryWriteVirtualization`/`unvirtualizedResources` if allowed; otherwise the extension talks to the agent on `127.0.0.1` with a pairing token; and/or ship the native host via the direct installer |
| **S12** | **Cloudflare relay end-to-end:** latency and reliability of Worker → DO → WebSocket delivery and Worker → APNs from production, incl. from Lagos to the nearest edge | 1k messages between `baton-cli` devices on two networks; 100 sandbox pushes | p95 message delivery < 1 s; APNs accepted ≥ 99% | Tune DO placement (location hints); if APNs via `fetch` misbehaves, move only the APNs sender to a tiny HTTP/2 service and keep the rest on Workers |

Record each result in `docs/spikes/S<n>.md` (device, OS build, network, result, video) and update §7/§8/§3 via ADR.

---
## 15. UX copy (key strings; keep it short, plain, no jargon)

| Surface | Copy |
|---|---|
| Desktop pair window | **"Pair your iPhone"** · "Open your iPhone camera and point it here." · small: "Code refreshes in 9:41" |
| Desktop SAS confirm | "**Raynan's iPhone** wants to pair. Does it show **482 913**?" [Pair] [Not my phone] |
| iPhone SAS | "Check that your PC shows **482 913**, then click Pair on your PC." |
| Paired | iPhone: "You're set. Lock your PC and pick up your phone; it'll be here." Desktop toast: "Baton is ready. Lock your PC to try it." |
| Continue notification | Title: "Continue: How Rust ownership works" · Body: "62% · from Raynan-PC" · Actions: **Open**, **Not now** |
| Continue (video) | "Continue: *Video title*" · "12:41 of 38:10 · from Raynan-PC" |
| Live Activity | "Continue on iPhone" · title · progress bar · "from Raynan-PC · now" |
| Generic fallback push | Title "Baton" · Body "New item from your PC" |
| Clip notification | "Copied on PC · 24 characters" · Action **Copy** (preview on: "Copied on PC: 'Meeting at 3pm…'") |
| File notification | "Raynan-PC sent *photo.jpg* · 3.2 MB" · **Save** |
| iPhone → PC toast | "From iPhone: *Title*" [Open] · Clip: "Pasted from iPhone ✓" (quiet) |
| Trigger card (Action button) | "**Send to your PC in one press.** Hold your Action button to send what you copied or what's open." [Set up · 3 taps] [Not now] |
| Trigger card (Control) | "**Add Baton to Control Center.** Swipe down, tap Baton, and it's on your PC." [Show me] |
| Trigger card (Back Tap) | "**Double-tap the back of your iPhone** to send to your PC." [Set up] |
| Honest limit (Settings › How it works) | "iOS doesn't let any app run all the time or read your clipboard on its own. That's why Baton uses one tap on iPhone. Everything else is automatic. When the Baton app is open on the same Wi-Fi, things go straight to your PC; otherwise they go through an encrypted relay that can't read them." |
| Connection status (Settings) | "Connected over Wi-Fi" · "Connected via relay" · "Direct Wi-Fi is off, so Baton is using the relay. [Fix]" |
| Errors | "Your iPhone isn't reachable. Open Baton on it once." · "Notifications are off, so Baton can't show your PC items. [Open Settings]" · "Offline. Will send when you're back online." |

---

## 16. Repo, CI, testing

### 16.1 Monorepo layout
```
baton/
├─ BATON.md  PROGRESS.md  README.md  LICENSE*  CONTRIBUTING.md (DCO)
├─ docs/  adr/  spikes/  threat-model.md  device-test-matrix.md
├─ proto/baton/v1/*.proto
├─ config/identity.{toml,json}        # names, bundle IDs, domains (batonsync)
├─ crates/
│  ├─ baton-core/      # protocol, crypto, pairing, LAN link, router, outbox, restore_url
│  ├─ baton-ffi/       # UniFFI bindings + build script → BatonCore.xcframework
│  ├─ baton-platform/  # clipboard/idle/lock/keystore traits + win/linux impls
│  ├─ baton-nmhost/    # native messaging host
│  └─ baton-cli/       # dev tool: fake device (pair, send, receive; --lan / --relay)
├─ server/             # Cloudflare Worker (TypeScript): src/, durable objects, wrangler.toml, test/
│  └─ mock-apns/       # tiny HTTP server asserting APNs headers/payloads for tests
├─ apps/
│  ├─ desktop/         # Tauri 2 (src-tauri/ + ui/ Svelte) + packaging/msix/
│  ├─ ios/             # project.yml (XcodeGen), App/, NSE/, Widgets/, Share/, Intents/, AppClip/, SafariExt/
│  └─ android/         # (post-MVP)
├─ extensions/browser/ # WXT MV3 (chrome, edge, firefox; safari later)
├─ web/                # Cloudflare Pages: landing, AASA, /p fallback, privacy, device-capabilities.json
└─ .github/workflows/  # ci-rust ci-server ci-desktop ci-ios ci-ext release deploy-server
```

### 16.2 CI (GitHub Actions)

| Workflow | Runner | Does |
|---|---|---|
| `ci-rust` | ubuntu-latest, windows-latest | fmt, clippy `-D warnings`, `cargo test --workspace`, `cargo deny`, LAN end-to-end tests between `baton-cli` instances (loopback mDNS) |
| `ci-server` | ubuntu-latest | `tsc`, lint, Vitest in the Workers pool (DOs, R2, alarms), mock APNs, then relay end-to-end tests: `wrangler dev` + two `baton-cli --relay` devices |
| `deploy-server` | ubuntu-latest, manual/tag | `wrangler deploy --env dev\|prod`. **Disabled until `CLOUDFLARE_API_TOKEN` exists** |
| `ci-desktop` | windows-latest, ubuntu-latest | Tauri build → **MSIX** (test-signed in CI) + NSIS installer + AppImage/.deb artifacts. Store submission is manual via Partner Center (or the Microsoft Store submission API once credentials exist) |
| `ci-ext` | ubuntu-latest | lint, unit tests, Playwright with the extension loaded (capture/restore on fixture pages), zips for the Chrome, Edge and Firefox stores |
| `ci-ios` | **macos-latest** (Xcode 26+) | Build the xcframework (`cargo build --target aarch64-apple-ios{,-sim}` + `x86_64-apple-ios`), `xcodegen`, `xcodebuild build test` on the simulator, no signing. The signed TestFlight lane (fastlane / `xcodebuild -exportArchive` with an App Store Connect API key) is **disabled until secrets exist** |
| `release` | tags | Builds all artifacts, checksums, draft GitHub release |

- **Cloud Linux agents can't build, sign or run iOS.** They can write Swift, XcodeGen config and tests, but verification happens only on `ci-ios` (macOS) or on a Mac. Keep iOS PRs small and CI-checked.
- macOS runner minutes are billed at a premium on private repos. Run `ci-ios` only on path changes (`apps/ios/**`, `crates/baton-core/**`, `crates/baton-ffi/**`) and cache cargo and DerivedData.

### 16.3 Testing strategy

| Layer | What |
|---|---|
| Core unit + property | Proto round-trips, HPKE/Noise/SAS vectors, STREAM chunking (proptest), restore-URL builder table tests (YouTube variants, existing `#hash`, tracking params), router decisions (table-driven over §3.2) |
| LAN end-to-end | Two or three `baton-cli --lan` devices: discovery, `Noise_KK` link, unknown peer rejected, file stream + resume, kill-the-link → relay fallback |
| Server | Vitest in the Workers pool: device registration, `Baton-Sig` checks, rendezvous, peer grants (unpaired sender rejected), mailbox queue vs WebSocket delivery, TTL alarms, R2 blob lifecycle, rate limits, mock APNs (headers, payload shape, 410 handling), and "no plaintext in storage" assertions |
| Desktop | Platform trait fakes. Windows clipboard listener + concealed-format filter test (sets formats programmatically). Lock/idle simulation through the trait. MSIX install smoke test on `windows-latest` |
| Extension | Playwright: fixture article → capture → fragment uniqueness → restore. Video fixture → time capture. Native-messaging host stub |
| iOS | XCTest: capability lookup and trigger-choice logic (table-driven over every row of §8.2/§8.3), FFI vector tests, deep-link parsing, NSE render from a fixture payload. Snapshot tests of Live Activity / widget views |
| E2E (manual, scripted) | `docs/device-test-matrix.md` checklist per release + demo flow, run once on the same Wi-Fi and once with the phone on mobile data |

### 16.4 Real-device test matrix (minimum before public TestFlight)

| Device class | Example | Why |
|---|---|---|
| No Action button, no DI, oldest | iPhone XR or XS (iOS 18) | Floor device, Back Tap path, no iOS 26 |
| Home button | iPhone SE (2nd or 3rd gen) | Touch ID, small screen, no DI |
| No Action button, DI | iPhone 14 Pro or 15 | Control Center primary, DI presentation |
| Action button, iOS 17.x | iPhone 15 Pro kept on 17.x (if obtainable) | Shortcut-only Action path |
| Action button, no DI | iPhone 16e or 17e | Notch + Action |
| Action button + DI, current | iPhone 16/17 family on iOS 26/27 | Main path |
| Foldable, no Action button | iPhone Duo (if accessible) | The "newer ≠ Action button" case |
| Desktop | Windows 11 24H2 (Store MSIX + NSIS), Windows 10 22H2, Ubuntu 24.04 GNOME (Wayland), Fedora KDE (Wayland), an X11 session | Clipboard, idle, lock and packaging variance |
| Networks | Home Wi-Fi, mesh Wi-Fi, phone hotspot, client-isolated Wi-Fi, phone on mobile data | LAN vs relay switching |
| Browsers | Chrome, Edge, Firefox, Brave | Extension + native messaging |
| Password managers | KeePassXC, Bitwarden, 1Password | Concealed-clipboard filtering |

---

## 17. Milestones (4-week MVP)

### M1 · Week 1: Foundations, pairing, both transports
- [ ] Monorepo scaffold, `config/identity`, licences, CI green (`ci-rust`, `ci-server`, `ci-ext`, `ci-ios` simulator build)
- [ ] `baton-core`: proto, HPKE seal/open, Noise IKpsk2 pairing + SAS, `Noise_KK` LAN link, transport router, outbox, restore-URL builder, test vectors
- [ ] `server/`: Worker + `DeviceDO` + `RendezvousDO`, peers/grants, mailbox + WebSocket, R2 blobs, APNs sender (mock in CI; real sandbox once deployed with the key), `wrangler.toml` dev/prod
- [ ] `baton-cli` fake device with `--lan` and `--relay`
- [ ] Windows desktop: tray, pair window with QR + SAS confirm, keystore, LAN listener + mDNS, lock/idle detector, clipboard watcher + concealed filter (logged, not yet sent). MSIX + NSIS packaging in CI
- [ ] iOS skeleton: XcodeGen project with App, NSE, Widget, Share targets. The xcframework builds. Scanner + universal-link parsing. Pairing UI (LAN first, rendezvous fallback). `DeviceCapabilities.json` + trigger-choice logic with tests
- [ ] Spike harnesses for S1, S3, S4, S6, S10, S11, S12 ready

**Acceptance:** two `baton-cli` devices pair over the LAN and, separately, through a locally running Worker; they see matching SAS and exchange E2E `Continue`/`Clip` (server storage holds only ciphertext, checked by a test). Killing the LAN link mid-session falls back to the relay in ≤ 2 s. The Windows CI artifacts install, show the QR and pair with `baton-cli`. The iOS app builds and its unit tests pass on `macos-latest`. `PROGRESS.md` lists every blocker that needs an account or secret.

### M2 · Week 2: Desktop → iPhone "Continue"
- [ ] Extension (Chrome/Edge first, Firefox next): TabState, fragment, media. Native host + agent IPC
- [ ] Handoff detector → `Continue` / `Withdraw`, cooldowns
- [ ] Deploy the Worker to `dev`. Server → APNs alert (mutable) + Live Activity start/update/end
- [ ] iOS: NSE decrypt/render, CONTINUE category, Live Activity UI (DI + Lock Screen), deep link → restore URL, Withdraw handling
- [ ] Run S1, S2, S9, S12 and record the results

**Acceptance:** on a real iPhone, lock the PC on an article at 60% → a card within 10 s → tap → Safari at the same paragraph. YouTube at 12:41 → the YouTube app at 12:41 ± 2 s. Unlocking the PC within 2 min removes the card. Works with the phone on the same Wi-Fi **and** on mobile data.

### M3 · Week 3: iPhone → PC, clipboard, files, triggers
- [ ] Share extension (URL, text, image, file): LAN if reachable, else relay/R2 with background upload. Desktop toast + pending restore
- [ ] `SendToPCIntent`, App Shortcuts, Control (iOS 18+), Lock Screen widget. **Device-aware trigger card** (§8)
- [ ] Clipboard PC → iPhone (mode a + hotkey) with a Copy action. iPhone → PC via the intent. Run S3, S4, S6, S10
- [ ] File drop both ways: LAN stream when the app is open, R2 blob otherwise, LAN upgrade on open, Files/Photos save, `~/Downloads/Baton`

**Acceptance:** share a photo from iPhone → on the PC in < 5 s, both at home (LAN) and away (relay). A 200 MB video goes PC → iPhone over LAN with the app open, and a 50 MB file goes via R2 when the phone is away. Copy on the PC + lock → phone card with **Copy** → paste works. The trigger card shows the right primary option on at least three device classes from §16.4 (Action, non-Action iOS 18+, Home-button), checked against the table-driven tests.

### M4 · Week 4: Seamless onboarding, Store, polish, Linux, demo
- [ ] Onboarding timed < 90 s (Path B at least; Path A if S7 passes). JIT permission primers. Settings › How it works
- [ ] **Microsoft Store submission** of the MSIX (S11 result applied) + direct download on the landing page
- [ ] Linux desktop (X11 + KDE Wayland; GNOME per S8). Firefox extension
- [ ] Optional: Safari Web Extension (S5)
- [ ] Opt-in telemetry, privacy policy, App Review notes, web `/p` + AASA on Cloudflare Pages, landing page
- [ ] Load test the server. Revisit minimum iOS (§8.5)
- [ ] Record the demo video

**Acceptance:** 3 fresh testers onboard in < 90 s median without help. The full matrix checklist passes on ≥ 4 iPhone classes + Windows 11 + one Linux distro, on both LAN and relay. TestFlight build submitted (if the Apple account exists). Store listing submitted.

### Demo video script (≈ 25 s, vertical + horizontal cuts)
1. **0–5 s.** Windows laptop, reading a long article, scrolled to the "Ownership" section. Text on screen: *"Reading on my PC…"*
2. **5–8 s.** Press ⊞+L (lock). Pick up the iPhone. The Lock Screen already shows **"Continue on iPhone: How Rust ownership works · 62%"**.
3. **8–11 s.** Tap → Safari opens *at the same paragraph* (the highlighted text-fragment flash). Text: *"Same paragraph. One tap."*
4. **11–16 s.** Laptop: a YouTube video at 12:41 → lock → phone: tap → the YouTube app at 12:41.
5. **16–20 s.** Copy an address on the PC, lock → phone notification **Copy** → paste into Maps.
6. **20–24 s.** On the phone, press the **Action button** (or double back-tap on an older iPhone, side by side) → a photo/link lands on the PC with a toast.
7. **24–25 s.** End card: *"Baton: Handoff for iPhone + Windows/Linux. Works anywhere. Open source. End-to-end encrypted."*

---
## 18. Telemetry (opt-in, privacy-preserving)
- **Off by default.** Asked once at the end of onboarding: "Share anonymous counts to help improve Baton? No URLs, titles or clipboard, ever."
- Events (counts and buckets only): `pair_completed{duration_bucket, path:A|B}`, `continue_sent{reason}`, `continue_opened{latency_bucket}`, `trigger_setup{kind}`, `spike_metric{id, ok}`, `error{code}`. A random install ID rotated every 30 days. No device or peer IDs and no IPs stored. Batches are sent daily to `POST /v1/telemetry` on a separate endpoint. Self-hosted servers have it disabled by default.
- Crash reports: opt-in, stack traces only, no payloads.

## 19. Out of scope (MVP) and later phases

**Out of the MVP:** BLE channel and ANCS notification mirroring (phase 2) · keyboard extension · photo auto-sync · macOS app and Safari-desktop extension · Android app (starts after M4) · multi-user/family groups · payments/Pro tier (IAP later per 3.1.1/3.1.3(b)) · Wi-Fi Aware / EU DMA accessory paths · auto-open on iPhone without a tap (impossible) · zero-tap iPhone clipboard (impossible) · document/app state beyond browser tabs and video.

**Later phase: accounts and sign-in.** Only once pairing-based identity is solid. Candidates: one sign-in that links all your devices at once (multi-device sync groups instead of pairing each pair), off-network history (encrypted Continue and clip history kept longer than 24 h), recovery when a phone is lost or replaced, and a Pro tier (bigger files, longer retention). Content stays end-to-end encrypted; the account only holds device lists and wrapped keys. Sign in with Apple becomes required on iOS if any third-party login is offered.

## 20. App Review guardrails (bake into design & review notes)

| Guideline | How we comply |
|---|---|
| 2.5.4 background modes | No background modes in the MVP. BLE only in phase 2, and only if the PC truly acts as a BLE device |
| 4.5.4 push content | E2E + generic fallback text + clip previews redacted by default. No marketing pushes |
| 4.2.3(i) standalone value | The iOS app offers clip history, a share inbox, and phone ↔ phone pairing (two iPhones can pair). Review notes include a demo video + a hosted test desktop build |
| 4.4.1 keyboards | Keyboard deferred. If built: works without Full Access and doesn't launch other apps |
| 3.1.x payments | No payments in the MVP. A future Pro uses IAP (3.1.1). Web sales allowed for multiplatform services (3.1.3(b)) |
| 5.1 privacy | Privacy policy, accurate privacy label, clear clipboard disclosure screen |
| 2.3.10 metadata | Describe it as "your computer/PC". Show real functional screenshots only |

## 21. Open questions for Raynan
1. **Apple Developer Program** ($99/yr): do you have one, and is it individual or organisation? It's needed for APNs keys, App Groups, Live Activity push testing, Associated Domains, the App Clip and TestFlight. Until then the iOS work stays simulator-only and the spikes wait.
2. **Mac access:** do you own a Mac, or should we plan on a rented cloud Mac? CI covers builds, but the S1–S10 spikes need Xcode on a real iPhone (provisioning, device logs).
3. **Domain:** register `batonsync.app` (or another) on Cloudflare now. It's needed for universal links (AASA), the Worker's custom domain and the landing page. Availability isn't checked yet.
4. **Clipboard default:** is "send when I leave my PC" (mode a + hotkey, §7.4) the right default, or hotkey-only?
5. **Licence (default unless you object):** Apache-2.0 for clients + core, AGPL-3.0 for `server/`, plus a DCO. The trade-off is below.

Already settled, no action needed: the name is **Baton**; Windows signing and distribution go through your existing Windows developer account and Store presence (localpdf), so only the app name needs reserving in Partner Center; hosting is Cloudflare (one account for Workers, Durable Objects, R2 and Pages).

**Licence trade-off**

| Option | Pros | Cons |
|---|---|---|
| MIT/Apache-2.0 for everything | Max adoption and contributions, no App Store friction, companies can embed it | Anyone can run a closed hosted competitor on your server code |
| AGPL-3.0 for everything | Hosted forks must publish changes, a strong "open" signal | GPL-family licences conflict with App Store terms for *third-party* contributions (the VLC 2011 precedent). Needs a CLA so you can relicense for the App Store. Scares some contributors |
| **Recommended: Apache-2.0 for clients + core, AGPL-3.0 for `server/`**, plus a DCO | Clients ship to stores freely. Hosted server forks must share changes | Two licences to explain (one paragraph in the README) |

---
## 22. Definition of Done (any PR)
- [ ] Tests added or updated, and CI green on the affected workflows
- [ ] No secrets, keys or tokens in the diff (`gitleaks` in CI)
- [ ] `PROGRESS.md` updated (done / next / blocked)
- [ ] Any user-visible string comes from §15 or is added there
- [ ] Spec deviations recorded as an ADR
- [ ] Behaviour checked on both the LAN and relay paths where relevant

---

# Appendix A: Research. How seamless can Windows/Linux ↔ iPhone get?

*Research brief for Raynan, 23 Oct 2026. Sources are linked inline. Anything marked **[unverified]** is an inference or something not confirmed in a primary source, so prototype it before relying on it. Section references like §3 point to Part 2 of this document.*

---

## A1. Conclusion first: how close can it get?

**Short answer:** it can't match AirDrop or Handoff with "just download an app" on iPhone. It *can* get to **"appears by itself, one tap to act"** for anything going *to* the iPhone, and to **one or two taps** for anything coming *from* the iPhone. Windows, Linux, Mac and Android can be close to fully zero-tap, with some limits on Android. Setup is a one-time job of about 60–90 seconds, not zero.

Why the iPhone is the ceiling:

1. **No persistent background process.** iOS suspends apps. Background time comes in small, system-controlled slices: BGTaskScheduler, roughly 10–30 s wakes from pushes or BLE events, and user-started continued tasks. A suspended app can't keep a socket, an mDNS listener or a Wi-Fi Aware listener open ([WWDC25 "Finish tasks in the background"](https://developer.apple.com/videos/play/wwdc2025/227/), [Apple forums on NWListener and suspension](https://developer.apple.com/forums/thread/840808), [Wi-Fi Aware and background](https://developer.apple.com/videos/play/wwdc2025/228/)). LocalSend and KDE Connect both hit this wall: on iOS you have to keep the app open to receive ([LocalSend #1468](https://github.com/localsend/localsend/issues/1468), [#2621](https://github.com/localsend/localsend/issues/2621), [KDE Connect iOS](https://github.com/KDE/kdeconnect-ios)).
2. **No reading the clipboard in the background, and reading it prompts.** Since iOS 16, a programmatic paste of another app's content shows "Allow Paste". Only user-initiated paste (the system Paste menu, ⌘V, `UIPasteControl`) avoids the prompt ([WWDC22 privacy](https://developer.apple.com/videos/play/wwdc2022/10096/), [UIPasteControl](https://developer.apple.com/documentation/uikit/uipastecontrol)). "Copy on iPhone, it's on the PC" with zero taps is **impossible**.
3. **Apple's own Handoff and Universal Clipboard are closed.** They need the same Apple Account, rely on encrypted BLE advertisements plus AWDL, and use keys synced through iCloud Keychain ([Apple Handoff security](https://support.apple.com/guide/security/handoff-security-secf78dbe639/web), [Stute et al., USENIX Sec '21](https://www.usenix.org/system/files/sec21-stute.pdf)). A non-Apple device can't join.
4. **No "phone unlocked" event.** `protectedDataDidBecomeAvailable` only fires while your app is already running, and it doesn't wake a suspended app ([Apple forums](https://developer.apple.com/forums/thread/751215)). Shortcuts has no unlock trigger ([Apple Shortcuts automation guide](https://support.apple.com/guide/shortcuts/enable-or-disable-a-personal-automation-apd602971e63/7.0/ios/17.0)).

**What gets you most of the way back:**

- **Push-driven UI that shows up without the app running.** Alert pushes, **Live Activities started by push** (push-to-start, iOS 17.2+) and Lock Screen widgets mean "Continue: *article*, 60%" is already on the Lock Screen when you pick up the phone. One tap opens it ([ActivityKit push docs](https://developer.apple.com/documentation/activitykit/starting-and-updating-live-activities-with-activitykit-push-notifications.md)).
- **The desktop decides when a handoff is happening, not the phone.** The desktop knows when you go idle or lock the screen. That moment is when the "continue" card gets pushed to the phone.
- **Extensions inside iOS where the user already is:** the Share Sheet, a Safari Web Extension (which can capture the active tab's URL, scroll position and video time), Control Center controls, the Action button or Back Tap running an App Intent, and optionally a custom keyboard that can insert desktop clips with one tap.
- **BLE as a wake-up channel [promising, unverified end-to-end].** An iOS app with Core Bluetooth background modes is woken for BLE events from a device it's connected to, and can be relaunched through state restoration ([Core Bluetooth background](https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html), [TN3115](https://developer.apple.com/documentation/technotes/tn3115-bluetooth-state-restoration-app-relaunch-rules)). A PC acting as a BLE peripheral could nudge the phone with no APNs round trip and also sense proximity. That's the most "magic" lever available, and also the riskiest one with App Review.

**Rough seamlessness score against Apple Continuity (10 = same):** Desktop ↔ desktop 9. Android ↔ desktop 7–8. **Desktop → iPhone 7** (it appears on its own, one tap to act). **iPhone → desktop 5–6** (Share Sheet, Safari extension or Action button; the clipboard needs a tap).

**The DMA doesn't change this for a desktop app in 2026.** The EU-only interoperability features are built for *connected accessories*: notification forwarding to wearables, proximity pairing, Wi-Fi network sharing and accessory background execution. They need AccessorySetupKit pairing, organization-level developer access and usually EU-located users ([Apple proximity pairing](https://developer.apple.com/proximity-pairing/), [MacRumors iOS 26.5](https://www.macrumors.com/2026/05/11/ios-26-5-features/), [EC factsheet, May 2026](https://digital-markets-act.ec.europa.eu/factsheet-how-dma-making-smartphones-better-interoperability-and-data-portability-case-studies-2026-05-11_en)). Wi-Fi Aware is available worldwide on iOS 26, but **Windows has no Wi-Fi Aware/NAN API**, and on Linux it depends on the driver ([Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/2284386/does-windows-have-plans-to-support-wifi-aware), [nl80211](https://docs.kernel.org/netlink/specs/nl80211.html)). Those features are worth watching, but they aren't your foundation.

---

## A2. Feature × platform matrix

Legend: **0** = zero-tap (it just happens), **1** = one tap or one gesture, **2** = two or three taps, **✕** = impossible with public APIs, **~** = possible but unreliable or opportunistic.

### A2a. Continue on other device

| Flow | Win/Linux/Mac desktop | Android | iPhone |
|---|---|---|---|
| **Capture** the current browser tab, scroll and video time | **0** (browser extension + native agent) | **~/1** (no extension API in Chrome Android. Capture via Share target, or the app's own Accessibility service, which risks Play review) | **0 in Safari** with a Safari Web Extension the user allowed on all sites **[verify reliability]**. **1** through the Share Sheet. **✕** for other apps' state (YouTube app, Chrome iOS, which has no extensions) |
| **Capture** the open file or document page | **0** for supported apps (Acrobat/Office/VS Code via plugins or window title, best effort) | ~ | ✕ (no visibility into other apps) |
| **Surface** a "Continue X" card on the target | **0** (tray chip or toast) | **0** (notification or bubble) | **0** to *appear* (alert push or Live Activity push-to-start, Lock Screen widget update) |
| **Open it in place** (URL + scroll + timestamp) | **1** (or 0 with an auto-open setting) | **1** | **1** (tap the notification or Live Activity, then a universal link or Safari). **✕** for auto-open without a tap |
| **Detect the handoff moment** | **0** (idle, lock, focus events) | **0** (screen-on broadcast to a foreground service) | ✕ on the phone (no unlock event). Workaround: **the desktop detects** it and pushes. BLE proximity **~** |

### A2b. Universal clipboard

| Flow | Desktop | Android | iPhone |
|---|---|---|---|
| Copy **on desktop**, available on the other device | **0** to watch and send (Windows clipboard listener, X11/Wayland watchers vary, macOS NSPasteboard: macOS 15.4+ previewed a read prompt that is reportedly *not* on by default in macOS 26 ([M. Tsai](https://mjtsai.com/blog/2025/05/12/pasteboard-privacy-preview-in-macos-15-4/)) **[watch this]**) | **0** to receive and write while the app is visible. A background write may be rejected ([Android 10 privacy](https://developer.android.com/about/versions/10/privacy/changes)) **~** | **1**: a notification "Copied on PC: …" with a **Copy** action, or tap a clip in your custom keyboard to insert it directly. **✕** for zero-tap pasteboard updates (no reliable background runtime; silent pushes are throttled to about 2–3 an hour ([Apple](https://apple-docs.everest.mt/docs/usernotifications/pushing-background-updates-to-your-app/))) |
| Copy **on phone**, available on desktop | n/a | **1** (Android 10+ blocks background clipboard reads unless you're the default IME or the focused app ([Android](https://developer.android.com/about/versions/10/privacy/changes))). An own IME gets to ~0 | **1–2**: Share Sheet → "Send to PC" on selected text, Back Tap/Action button → Shortcut (Get Clipboard → app intent), or a `UIPasteControl` button in the app or keyboard. **✕** for automatic |

### A2c. File drop

| Flow | Desktop | Android | iPhone |
|---|---|---|---|
| Desktop → phone | **1** (right-click "Send to phone" or drag onto the tray) | **0** to receive (foreground service, LAN) | **1** to accept from a notification. The download can run with a **background URLSession** from a relay. LAN peer-to-peer only while the app is open (**✕** background listener) |
| Phone → desktop | **0** to receive | **1–2** (Share target) | **2** (Share Sheet → your extension, which uploads with a background session) |
| Auto-sync new photos | **0** | **0** (foreground service + MediaStore) | **~** (only when the app runs: opportunistic BGAppRefresh, opening the app, or BGContinuedProcessingTask, which is *user-started* and shows progress UI ([BGContinuedProcessingTask](https://developer.apple.com/documentation/backgroundtasks/bgcontinuedprocessingtask.md))) |

### A2d. Setup (one-time)

| | Desktop | Android | iPhone |
|---|---|---|---|
| Install + pair | Installer + browser extension (2 installs) | App + QR scan + notification permission + battery-optimization exemption | App + QR scan + notification permission + Local Network prompt (if LAN) + *optional*: enable the Safari extension (Settings/Safari, about 4 taps), keyboard with Full Access (about 5 taps), Back Tap/Action button Shortcut (about 6 taps) |
| "Just download an app"? | Nearly | Nearly | **No**: the core works after app + QR + notifications. The magic extras need extra setup, so guide people through it with deep links |

---

## A3. iOS constraints in detail (what's allowed in 2026)

### A3.1 Background execution
- **BGAppRefreshTask / BGProcessingTask:** opportunistic and coalesced by the system, with energy as the main constraint. Never use them for real-time sync ([WWDC25-227](https://developer.apple.com/videos/play/wwdc2025/227/)).
- **beginBackgroundTask:** a short grace period to finish work after the app is backgrounded.
- **BGContinuedProcessingTask (new in iOS 26):** work the user starts in the foreground, such as "send these 40 videos", can keep going in the background with system progress UI (a Live Activity) and a cancel button. The system can still end it under resource pressure ([docs](https://developer.apple.com/documentation/backgroundtasks/bgcontinuedprocessingtask.md)). **Great for big file drops.** It is not a daemon.
- **iOS 27 App Intents `LongRunningIntent`** runs longer than the usual intent limit ([WWDC26 App Intents](https://developer.apple.com/videos/play/wwdc2026/345/), [summary](https://blakecrosley.com/blog/app-intents-ios-27-background-execution)) **[I read secondary coverage only; check the session]**.
- **Background modes** (audio, location, VoIP, bluetooth-central/peripheral, and so on) may only be used for their intended purpose ([App Review 2.5.4](https://developer.apple.com/app-store/review/guidelines/)). Silent-audio or location keep-alives get apps rejected or pulled. PushKit VoIP pushes have to report a real CallKit call, so that's off-limits for sync.

### A3.2 Networking in the background
- A suspended app has no networking. `NWListener`/Bonjour listeners need to be stopped and restarted around suspension ([forums](https://developer.apple.com/forums/thread/840808)).
- **Background `URLSession`** transfers are handled by the system and **continue while the app is suspended or even terminated**. They're the right tool for relay uploads and downloads (a long-standing, documented API).
- **Local Network privacy:** Bonjour browse/advertise and LAN connections need the Local Network prompt and the `NSBonjourServices` declaration. A denied listener can look ".ready" while silently failing ([TN3179](https://developer.apple.com/documentation/technotes/tn3179-understanding-local-network-privacy.md)).
- **Multipeer Connectivity / Network framework peer-to-peer (AWDL):** Apple-to-Apple only, and only in the foreground. No use for Windows or Linux.
- **Wi-Fi Aware (iOS 26, worldwide):** peer-to-peer Wi-Fi to Apple *and third-party* devices. Pairing goes through DeviceDiscoveryUI (device picker + PIN) or AccessorySetupKit. Connections are on demand, with **both apps running** ([WWDC25-228](https://developer.apple.com/videos/play/wwdc2025/228/), [docs](https://developer.apple.com/documentation/wifiaware)). It's a plausible high-speed path for **iPhone ↔ Android** (Android has NAN APIs) **[cross-vendor interop unverified]**. It isn't a path for Windows, which has no NAN API ([MS Q&A](https://learn.microsoft.com/en-us/answers/questions/2284386/does-windows-have-plans-to-support-wifi-aware)), and Linux is driver-dependent (nl80211 has NAN commands, and [OpenNAN](https://github.com/seemoo-lab/opennan/) is research code).

### A3.3 Bluetooth LE
- With `bluetooth-central`/`bluetooth-peripheral` background modes, the app gets woken for BLE events (connection, notify/indicate, writes). State restoration can relaunch a system-terminated app, but **not after the user force-quits it** or turns off Bluetooth ([Apple guide](https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html), [TN3115](https://developer.apple.com/documentation/technotes/tn3115-bluetooth-state-restoration-app-relaunch-rules)).
- **iOS 26 change (worldwide):** Bluetooth accessories keep working after the user quits the companion app and reconnect when they come back in range ([EC factsheet](https://digital-markets-act.ec.europa.eu/factsheet-how-dma-making-smartphones-better-interoperability-and-data-portability-case-studies-2026-05-11_en)). **[Unverified whether this covers a PC acting as a generic GATT peripheral, or only accessories paired with AccessorySetupKit.]**
- A backgrounded iOS *peripheral*'s advertisement moves service UUIDs into Apple's "overflow area", which only iOS centrals scanning for that UUID can see. So the robust design is **iPhone = central, PC = peripheral** (or keep a connection open) ([Apple forums summary](https://developer.apple.com/forums/thread/11705)).
- **Bonus, the Phone Link trick:** once an iPhone is bonded to a PC over BLE, the PC can subscribe to **ANCS** (Apple Notification Center Service) and receive *all* iPhone notifications, with positive and negative actions, **with no iOS app running**. **AMS** gives now-playing info and media controls ([ANCS spec](https://developer.apple.com/library/archive/documentation/CoreBluetooth/Reference/AppleNotificationCenterServiceSpecification/Specification/Specification.html), [AMS spec](https://developer.apple.com/library/archive/documentation/CoreBluetooth/Reference/AppleMediaService_Reference/Specification/Specification.html)). That's how Microsoft Phone Link does iPhone notifications over BLE ([Phone Link setup](https://support.microsoft.com/en-us/windows/apps/phonelink/phone-link-requirements-and-setup)). It's a strong optional "notification mirroring" feature for your desktop agent.

### A3.4 Push notifications
- **Alert pushes** are delivered reliably and show without the app running. A **Notification Service Extension (NSE)** can decrypt an E2E payload before display. Apple raised the NSE memory limit in response to a DMA request, citing E2E use ([EC factsheet](https://digital-markets-act.ec.europa.eu/factsheet-how-dma-making-smartphones-better-interoperability-and-data-portability-case-studies-2026-05-11_en)).
- **Silent pushes** (`content-available`, priority 5) are best-effort. Apple says not to send more than **2–3 an hour**, and they're throttled ([Apple](https://apple-docs.everest.mt/docs/usernotifications/pushing-background-updates-to-your-app/)). Don't build sync on them.
- **App Review 4.5.4:** pushes "should not be used to send sensitive personal or confidential information" ([guidelines](https://developer.apple.com/app-store/review/guidelines/)). Clipboard contents are sensitive, so E2E-encrypt everything and show a generic or redacted preview by default.
- **APNs needs a server holding your Apple .p8 key.** The desktop can't send to APNs directly without that key, so a relay or push service is mandatory for iOS (for Baton, the Cloudflare Worker in §10).

### A3.5 Live Activities and widgets
- **Push-to-start (iOS 17.2+):** the server can start a Live Activity through APNs with a push-to-start token. The system starts it and briefly wakes the app ([docs](https://developer.apple.com/documentation/activitykit/starting-and-updating-live-activities-with-activitykit-push-notifications.md), [forums on about 10 s of runtime](https://developer.apple.com/forums/thread/764912)). It shows on the Lock Screen and Dynamic Island. **This is the closest thing to a Handoff icon.** Caveats **[verify]**: ActivityKit has update budgets, and Live Activities are meant for ongoing events ("Continue reading" is an arguable fit, so frame it as an "active session from your PC"), plus there are time limits on how long one stays up.
- **Widgets / Control Center controls (iOS 18+):** widgets can refresh from pushes or timelines. Controls run App Intents (one tap).

### A3.6 Shortcuts, App Intents, Action button, Back Tap
- Automations that can **run without confirmation**: Time of Day, Alarm, Sleep, Arrive/Leave, CarPlay, Email, Message, Transaction, **Wi-Fi**, **Bluetooth**, Watch Workout, **NFC**, **App** (opened/closed), Airplane, DND, Low Power, Battery, Charger, Sound Recognition. **No unlock trigger.** Focus, Keyboard, Screenshot and Notification ask first ([Apple guide](https://support.apple.com/guide/shortcuts/enable-or-disable-a-personal-automation-apd602971e63/7.0/ios/17.0), [analysis](https://worldofhelios.com/blog/iphone-shortcuts-automation/)).
- **iOS 27** adds automation triggers for screenshot creation, external keyboard connection and **notification receipts** with keyword filters ([Apple Platform Interoperability Report, Sept 2026](https://developer.apple.com/download/files/Platform-Interoperability-Report.pdf)). **[Unverified whether these can run without confirmation.]**
- Useful tricks:
  - "**When Safari/Chrome is opened → run 'Pull from PC'**" (App trigger, runs immediately).
  - "**When I connect to home Wi-Fi → sync**".
  - An **NFC sticker on the desk** that pulls the PC's current tab on tap (zero app-open friction).
  - **Back Tap / Action button** → "Send clipboard to PC". Whether Shortcuts' Get Clipboard triggers the paste prompt each time or once is **[unverified]**.

### A3.7 Share Sheet, Safari Web Extension, keyboard
- **Share extension:** the universal "send to PC" entry point for URLs, text, photos and files. Uploads hand off to a background URLSession.
- **Safari Web Extensions on iOS (15+):** content scripts run on sites the user allows. `tabs` works, with some APIs missing (windows, webRequest, context menus) ([Apple compatibility](https://developer.apple.com/documentation/safariservices/assessing-your-safari-web-extension-s-browser-compatibility.md), [WWDC21](https://developer.apple.com/videos/play/wwdc2021/10104/)). The extension can read `scrollY` and `<video>.currentTime` and hand them to the containing app through native messaging. **That makes it the one way to get near-zero-tap capture of the iPhone's current tab.** **[Verify: whether the native handler can make network calls reliably, and how often the non-persistent background page runs.]** The same extension can **restore** scroll position when the desktop's URL opens in Safari.
- **Custom keyboard with Full Access:** can read and write the pasteboard and use the network. App Review 4.4.1 rules apply: it must work without full access and can't launch other apps. Whether pasteboard reads from a keyboard trigger the iOS 16+ prompt is **[unverified]**. Inserting a desktop clip *as typed text* skips the pasteboard entirely, a real one-tap "paste from PC".

### A3.8 Handoff / NSUserActivity
- Same Apple Account, Apple devices only. `webpageURL` falls back to the browser on the receiving *Apple* device ([Apple support](https://support.apple.com/en-us/102426), [security guide](https://support.apple.com/guide/security/handoff-security-secf78dbe639/web)). **Non-Apple devices can't take part.** On a Mac target, you could still donate NSUserActivity so native Handoff works Mac↔iPhone alongside your system.

### A3.9 Universal Clipboard internals (why you can't join)
- On copy, the device sends an encrypted BLE advertisement. Peers find each other over AWDL/mDNS, authenticate with long-term keys synced through iCloud Keychain (pair-verify, ECDH), and transfer with ChaCha20-Poly1305 (TLS for large payloads) ([Apple security](https://support.apple.com/guide/security/handoff-security-secf78dbe639/web), [USENIX '21](https://www.usenix.org/system/files/sec21-stute.pdf)). The keys live in the user's iCloud Keychain, so third parties can't take part.

### A3.10 EU DMA status in Oct 2026 (what shipped vs. what's due)

Sources: [EC DMA portal](https://digital-markets-act.ec.europa.eu/developer-portal/interoperability/apple-specification-proceedings-interoperability-connected-devices-and-request-based-process_en) and [EC factsheet, 11 May 2026](https://digital-markets-act.ec.europa.eu/factsheet-how-dma-making-smartphones-better-interoperability-and-data-portability-case-studies-2026-05-11_en).

| Feature | Status | Relevant to you? |
|---|---|---|
| Notification forwarding to third-party *connected devices* (AccessoryNotifications, beta in 26.3, shipped in 26.5, EU only, one device at a time) | Shipped ([MacRumors](https://www.macrumors.com/2026/05/11/ios-26-5-features/), [docs mirror](https://apple-docs.everest.mt/docs/accessorytransportextension/receiving-ios-notifications-on-an-accessory/)) | Built for wearables over BLE plus an AccessorySetupKit pairing. Whether a PC could register as an "accessory" is **[unverified, probably not intended]**. ANCS already gives you read-and-act notifications worldwide |
| Proximity-triggered pairing (iOS 26.5, EU, org account, self-certified accessory) | Shipped ([Apple](https://developer.apple.com/proximity-pairing/)) | For hardware makers. Not you |
| Wi-Fi Aware 4.0 (iOS 26, worldwide) | Shipped | Android yes. Windows no. Linux maybe |
| Close-range file transfer (AirDrop-alternative building blocks: AccessorySetupKit + Wi-Fi Aware + DeviceDiscoveryUI). "While the receiving app is open" due 1 Jun 2026; full measures in iOS 27 / end of 2026 | Partly shipped | Could eventually allow background receive for third-party AirDrop alternatives in the EU. **[Check the iOS 27 release notes; I found no confirmation that the full measures have shipped]** |
| Background execution for companion apps | Partly in iOS 26 (quit/BT reconnect). Full measures due iOS 27 / end of 2026 | Watch closely. Could greatly improve an EU-only experience **[unverified what iOS 27 delivered]** |
| Automatic Wi-Fi sharing (Wi-Fi Infrastructure, iOS 26.4, EU) | Shipped | No |

---

## A4. How Apple Continuity works, and can third parties copy it?

- **Stack:** BLE advertisements (encrypted "activity" beacons) → AWDL (Apple Wireless Direct Link, a proprietary peer-to-peer Wi-Fi mode) for bulk transfer → identity from Apple Account / iCloud Keychain ([USENIX '21](https://www.usenix.org/system/files/sec21-stute.pdf)).
- **Open reverse-engineered implementations:** SEEMOO's **OWL** (AWDL for Linux) is *no longer maintained*, experimental, and needs monitor mode plus frame injection on a dedicated Wi-Fi card. **OpenDrop** (AirDrop) depends on OWL and has reports of failing with newer iOS ([OWL](https://github.com/seemoo-lab/owl/blob/master/README.md), [OpenDrop](https://github.com/seemoo-lab/OpenDrop), [issue #119](https://github.com/seemoo-lab/opendrop/issues/119)). **Windows** can't do AWDL with standard drivers. **Not viable for a consumer app.**
- **Google Quick Share ↔ AirDrop (verified):** announced 20 Nov 2025, starting with the Pixel 10. Two-way, direct peer-to-peer, built by Google in Rust, *not* in partnership with Apple. It works only when the iPhone is set to AirDrop **"Everyone for 10 minutes"** ([Google blog](https://blog.google/products-and-platforms/platforms/android/quick-share-airdrop/), [Google security blog](https://blog.google/security/android-quick-share-support-for-airdrop-security/), [9to5Google](https://9to5google.com/2025/11/20/android-quick-share-airdrop-pixel-10/)). This shows interop is technically possible, but it took a platform owner with control of the radio stack. Expansion to other Android devices was promised. **[I did not verify the 2026 rollout status.]**
- **Takeaway:** don't speak Apple's protocols. Use standard transports (LAN TCP/QUIC, BLE GATT, relay) with your own app on the iPhone.

---

## A5. Prior art: how seamless it got and why it stalled

| Product | iPhone support | How seamless | What happened / lesson |
|---|---|---|---|
| **Intel Unison** | Yes (BLE + Wi-Fi) | Best-in-class iPhone↔Windows: texts, calls, files, notifications | **Discontinued 30 Jun 2025.** End of support 1 Jan 2026 (Lenovo Aura editions moved to Lenovo Smart Connect) ([Intel](https://www.intel.com/content/www/us/en/support/articles/000102120/software.html), [unison.intel.com](https://unison.intel.com/)). Corporate priorities, not tech, killed it. There's a gap in the market |
| **Microsoft Phone Link (iPhone)** | Yes, over BLE | Notifications (ANCS), calls, 1:1 texts while connected, file send from the Start menu | **No cross-device copy/paste, photo gallery or mirroring for iPhone.** No group messages or media in texts, and history clears on disconnect ([MS support](https://support.microsoft.com/en-us/windows/apps/phonelink/phone-link-requirements-and-setup), [Start menu page](https://support.microsoft.com/en-us/windows/apps/phonelink/mobile-device-in-start-menu)). Shows how far BLE plus system profiles go without an always-running iOS app |
| **Windows "Resume" / Cross-Device Resume** | **Android only** (Continuity SDK, limited access) | App task handoff phone → PC | [MS Learn](https://learn.microsoft.com/en-us/windows/apps/develop/windows-integration/cross-device-resume-overview). Microsoft itself didn't do iPhone |
| **Android 17 "Continue On"** | Android ↔ Android (phone → tablet first) | System-level handoff with app state | [Android dev](https://developer.android.com/develop/better-together/continue-on). It confirms the UX you want, but only inside one ecosystem |
| **KDE Connect iOS** | Yes (App Store, 0.5.x in 2025) | Clipboard, file/URL share, remote input | The app has to be open. Background disconnects; no notification sync ([repo](https://github.com/KDE/kdeconnect-ios), [App Store](https://apps.apple.com/us/app/kde-connect/id1580245991)). It's a LAN-only design, with no push or relay |
| **LocalSend** | Yes | Great LAN file transfer | The iOS app must be in the foreground to receive. Background receive is an open request ([#1468](https://github.com/localsend/localsend/issues/1468), [#2621](https://github.com/localsend/localsend/issues/2621)). Lesson: LAN-only can't wake an iPhone |
| **Snapdrop → PairDrop** | Web (any browser) | Zero install, but both tabs must be open | Snapdrop.net was sold to LimeWire. PairDrop is the community fork with pairing and self-hosting ([issue #663](https://github.com/SnapDrop/snapdrop/issues/663)) |
| **Blip** | iPhone + Windows/Mac | Share Sheet + Explorer integration, resumable, E2E with relay fallback | Well reviewed. Users mention iOS background limits ([blip.net](https://www.blip.net/), [Pocket-lint](https://www.pocket-lint.com/blip-app-airdrop-quick-share-alternative/)). Closest existing app for files, but no tab handoff or clipboard |
| **Pushbullet** | iOS app unpublished in 2020 (dispute over the Sign in with Apple requirement). The legacy push stopped working in 2021 | Once the gold standard for links and clips | [r/PushBullet](https://www.reddit.com/r/PushBullet/comments/eirc1m/not_available_on_ios/), [HN](https://news.ycombinator.com/item?id=23194469) (community sources). Lesson: iOS policy churn kills side projects, so keep up with App Review |
| **Join (joaoapps)** | No iOS app | Android/desktop power-user favourite | [joaoapps](https://joaoapps.com/join/) |
| **OnePlus Clipt** | Android/Chrome | Clipboard and file sync over Google Drive | Shut down 30 Sept 2023 ([9to5Google](https://9to5google.com/2023/09/28/oneplus-clipt-shutting-down/)) |
| **Dell Mobile Connect** | Had iPhone support | Mirroring/notifications | Discontinued years ago (Dell moved to Phone Link) **[from memory, not re-verified this session]** |

**Pattern:** every product that relied on a *running* iOS app (KDE Connect, LocalSend) feels broken on iPhone. The ones that felt good leaned on *system* channels: BLE profiles (Phone Link/Unison), the Share Sheet (Blip) and push. **Nobody has shipped desktop→iPhone tab handoff with scroll and timestamp via Live Activities**, so that's your gap.

---

## A6. Recommended architecture (research view)

The research recommendation became the architecture in **§3** (components, transport router, data flows), **§9** (protocol and crypto) and **§10** (the Cloudflare server). Notes from the research that aren't spelled out there:

- **Transport thinking:** LAN direct (mDNS discovery, encrypted TCP/QUIC) is fastest when both ends are awake. A blind store-and-forward relay always works and is the only thing that can wake iOS through APNs. BLE GATT (desktop as peripheral, iPhone as central) is a phase-2 option for small control messages and proximity. An existing peer-to-peer-with-relay library (for example iroh or libp2p) could replace hand-written NAT traversal later **[evaluate. I didn't benchmark these]**.
- **The handoff moment is detected on the desktop** (lock, idle with something continuable, lid close) and withdrawn on resume, so the Lock Screen card never feels stale. Phase 2: BLE RSSI on the desktop as an extra "phone moved away / came close" signal. On Android, a screen-on broadcast to the foreground service makes it fully automatic.
- **Restore on iPhone:** text-fragment URLs need no extension. The optional Safari extension restores the exact scroll fraction and video time. Native-app deep links where possible (YouTube `t=` is the big one).
- **Clipboard honesty:** "iOS won't let any app read your clipboard automatically" belongs in the UI. A custom keyboard (later) can type a desktop clip directly, a true one-tap "paste from PC".
- **Android (later):** desktop → phone writes while the app is focused, otherwise through a notification action. Phone → desktop through a notification action or an optional IME.

## A7. MVP scope (research view)

The four-week plan is in **§17**. The research's "prototype these first" list became spikes: Live Activity push-to-start reliability → **S1**, background pasteboard writes from a notification action → **S3**, Safari iOS extension reliability → **S5**, and whether Shortcuts' Get Clipboard prompts every time → **S4**. Building the plan surfaced three more: LAN from iOS extensions (**S10**), MSIX packaging (**S11**) and the Cloudflare relay end to end (**S12**).

---

## A8. Business and viral angle (brief)

- **Competition:** Phone Link (free, built into Windows, but weak on iPhone and no clipboard), KDE Connect and LocalSend (free, open source, foreground-only on iOS), Blip (files only), PairDrop (web). **Intel Unison's shutdown left a gap for iPhone + Windows users**, and nobody does *tab, scroll and video handoff* across ecosystems.
- **Positioning:** "Handoff for iPhone + Windows/Linux. Works anywhere, LAN when possible. Open source, end-to-end encrypted." The hatchzip formula: one universal pain ("I was reading this on my laptop…"), one clever trick (Lock Screen Live Activity plus paragraph-level restore), open source, and a 20-second video.
- **Distribution:** GitHub plus Show HN, r/iphone, r/Windows11, r/linux, r/selfhosted. Winget/Scoop/Flathub/AUR. Chrome Web Store and App Store listings themselves bring search traffic ("airdrop windows", "iphone to pc clipboard").
- **Monetization:**
  - Free core: LAN plus a limited relay.
  - **Pro** (about $3–5/month or a one-time purchase) for a hosted relay for large files, clip and session history, more devices, team or family groups.
  - Self-hosting stays free.
  - iOS unlocks must go through **in-app purchase** (3.1.1). A multiplatform service may also sell on the web if the IAP exists too (3.1.3(b)). Under US storefront rules you may link out (3.1.1(a)) ([guidelines](https://developer.apple.com/app-store/review/guidelines/)).
- **App Store review risks:**
  - 2.5.4 (background modes for intended use only): avoid any keep-alive tricks.
  - **BLE background modes** are defensible only if the PC really acts as a Bluetooth device the app talks to.
  - 4.5.4 (no sensitive data in pushes): E2E with generic previews.
  - 4.2.3(i) ("work on its own without another app"): give the iOS app standalone value, such as clip history, phone↔phone transfer and a web receiver.
  - 4.4.1 keyboard rules.
  - 5.1 privacy policy plus clear clipboard disclosures.
  - 2.3.10: don't name other platforms in the metadata. Even "Windows" in screenshots can draw scrutiny unless it's clearly functional. Many apps (Blip, Phone Link) do this fine **[low risk]**.
- **Existential risks:**
  - Apple builds its own (unlikely for Windows).
  - Microsoft improves Phone Link for iPhone (plausible).
  - APNs/ActivityKit policy changes.
  - Battery complaints.
  - Relay costs if files go viral: cap free relay size and prefer LAN.

---

## A9. Risks and open questions (ranked)

1. **The iOS "it appears but needs a tap" ceiling is permanent outside the EU.** Market it honestly ("one tap"). Don't promise "automatic".
2. **Live Activity suitability and budgets** for a "continue" use case **[prototype]**.
3. **Safari iOS extension capture reliability** **[prototype]**. If it fails, phone → desktop falls back to the Share Sheet, which is 2 taps.
4. **Linux Wayland clipboard and idle detection** vary by compositor (GNOME is the strictest).
5. **Security:** the clipboard often holds passwords and OTPs. Concealed-type filtering, E2E encryption, auto-expiry, and a "pause sync" toggle are table stakes. QR pairing needs a verification code to stop shoulder-surf MITM.
6. **DMA upside is uncertain and EU-only:** iOS 27 "full" background-execution and close-range file-transfer measures (due by end of 2026) could enable background receive for EU users **[check iOS 27 docs]**. Design the transport layer so a Wi-Fi Aware/accessory path can be added later.
7. **A server is a must for iOS** (APNs needs a server key, and remote use needs a relay), so Baton isn't "pure peer-to-peer". It's a content-blind Cloudflare Worker (§10) that holds only short-lived ciphertext, used alongside LAN direct whenever devices can reach each other, and self-hostable on any Cloudflare account.
8. **Cloudflare dependency:** pricing or limit changes (Durable Objects, R2), local `workerd` lacking HTTP/2 for APNs testing, and Durable Object placement latency from West Africa. Mitigations: keep the server small and portable, mock APNs locally, measure in S12, and keep a plan B of moving only the APNs sender to a tiny HTTP/2 service.
9. **LAN path variance:** client-isolated Wi-Fi, mesh networks, the iOS Local Network prompt being denied, Windows firewall prompts, and MSIX packaging limits (S10, S11). The relay fallback means none of these break the product; they only cost speed.
10. **Unverified items in this report:** BLE wake from a PC peripheral after iOS 26's quit-behavior change. Pasteboard writes from a background notification action. The keyboard paste prompt. Whether Shortcuts' Get Clipboard prompts. Whether iOS 27 notification-trigger automations run without confirmation. The iOS 27 DMA deliverables. Quick Share/AirDrop expansion beyond the Pixel 10. Dell Mobile Connect's status.

---

### Bottom line
Build it. The technical moat is the **desktop-driven handoff moment plus a Lock Screen Live Activity plus paragraph-level restore**, which nobody ships across ecosystems. The honest pitch is "one tap on iPhone, automatic everywhere else", and that still beats every alternative available since Intel Unison shut down.
