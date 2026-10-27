# Wendao minimal IronRDP fork

This branch is based on upstream [Devolutions/IronRDP](https://github.com/Devolutions/IronRDP) commit `3ca12ee63687980ace30cd6ed59c2ab4f57cf157` (2026-10-27). It is deliberately **not** a merge of our previous `codex/build-master-agent` integration branch.

## Downstream-only changes

- `FrameStream` agent IPC: request tag `41 (0x29)`, response `Payload::Frame` tag `18`. The long-lived connection acknowledges the subscription, then receives one latest **new** decoded frame for each byte `0x01` credit. Wire frame bytes are little-endian `u64 sequence`, `u16 width`, `u16 height`, `u32 BGR24 byte length`, and tightly packed BGR24 rows. Each RDP connection has a fresh watch sequence, and the decoded frame is shared for screenshot/stream conversion rather than deep-copied per subscriber.
- Per-session `ironrdp_pointer_software_rendering:i:0|1` controls remote pointer compositing; **default 0** for cursor-free Wendao OCR, explicitly opt in with 1.
- Agent/daemon opt-in RDP UDP: Cargo enables `ironrdp-client/udp`; `ironrdp_udp_transport:i:0|1` maps to `ConfigBuilder::with_udp_transport`. The underlying UDP implementation is entirely upstream.
- CLI prints metadata rather than raw `Frame` bytes; ActiveX reports `FrameStream` unsupported, leaving existing semantics unchanged.

**Do not import** prior custom EGFX, Progressive, SRL, or RDP-UDP packet/sequence/quantization patches. The Rust decoder and graphics stack stay on upstream implementations. Wendao currently requests `ironrdp_egfx:i:0` and may opt into upstream EGFX separately for an A/B validation.

## Wendao integration contract

The consumer is `littledotdot/wendao/airtest/core/rdp` on `master`:

- `client.py`: Unix-socket IPC for `Status`, `Screenshot`, `FrameStream`, mouse/key/wheel/text, plus per-credit frame waits.
- `protocol.py`: fixed little-endian request/payload tags, sequence, dimensions and BGR24 layout.
- `frame_runtime.py`: one stream per device, cached latest frame, frame sequence and connection-generation invalidation.
- `session.py`: daemon lifecycle, status checking, fresh-frame recovery, `ironrdp_pointer_software_rendering`, `ironrdp_udp_transport`, `ironrdp_egfx` properties.
- `device.py`: screen capture, snapshots, click, keyboard, NOW-based window focus and reconnect. Those existing generic agent requests remain handled by upstream.

## Verification (manually invoked, no automatic branch build)

```bash
cargo test -p ironrdp-rpc frame_stream_request_and_payload_round_trip
cargo test -p ironrdp-daemon frame_stream
cargo test -p ironrdp-daemon software_cursor_rendering_is_opt_in
cargo test -p ironrdp-daemon framebuffer_bgr_conversion_matches_retained_pixel_layout
cargo test -p ironrdp-agent
cargo build -p ironrdp-agent --release
```

On a macOS ARM64 Wendao workstation, replace the agent binary only after checking the target architecture and after the tests pass. Verify on Windows RDP: stream dimensions, colors, new-frame sequence, no repeated frames during idle periods, cursor off/on, typed input, NOW PowerShell/window focus, UDP on/off, manual resize, multiple roles and reconnects. Validate legacy/EGFX image correctness separately. The old branch/binary is the rollback fallback.

The inherited upstream `ci.yml` runs push CI only for `master` (PR and manual triggers also exist). This branch intentionally contains no automatic artifact-building workflow.
