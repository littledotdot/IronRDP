# Windows RDP UDP TLS EOF: RDPEMT sub-header fix (2026-10-29)

## Grounded evidence

Wendao's live report at `docs/research/rdp-udp-egfx-live-results-20261029.md`
(in `littledotdot/wendao`) records this on a Windows 11 RDP host:

- TCP legacy and TCP EGFX connected; UDP legacy failed once with TLS EOF.
- UDP + EGFX failed 4 of 4 attempts, after at least one log sequence
  `RDP-UDP handshake complete` → `TLS client handshake finished` →
  `Tunnel create response accepted` → `UDP transport established` →
  `Wendao reliable RDP-UDP DVC tunnel active`.
- The session subsequently emitted
  `UDP driver dropped without finishing, aborting the stream`,
  and Python surfaced `peer closed connection without sending TLS close_notify`.

**These messages do not establish event causality**: the UDP driver `Drop`
warning can result from cancellation after the TCP/TLS reader exits. Do not
treat the warning alone as an RDPEUDP2 ACK failure.

## Protocol comparison and concrete defect

FreeRDP's `libfreerdp/core/rdpemt.c` processes RDPEMT Tunnel Data
`RDP_TUNNEL_SUBHEADER` type `AutoDetectRequest`. `SubHeaderLength` and
`SubHeaderType` double as the `AutoDetectRequest` PDU's first two bytes.
See MS-RDPEMT 2.2.1.1.1 and MS-RDPBCGR 1.3.9 / 2.2.14.
Continuous RTT probes and bandwidth Start/Stop must be responded to over the
same reliable UDP sideband transport.

Before this patch, `ironrdp-rdpeudp-tokio::UdpTransport::recv_message`
correctly preserved `TunnelSubHeader`, but `ironrdp-client::rdp` called
`recv()` and discarded every sub-header. The existing
`ironrdp-session::AutoDetectResponder` could already answer the relevant
request variants, but only the regular TCP message-channel path used it.

This branch:

1. Changes the UDP receiver to `recv_message()`, keeping sub-headers and DVC
   payload together.
2. Reuses `AutoDetectResponder` on the UDP transport, maintaining a separate
   monotonic measurement window and byte count.
3. Encodes RTT and bandwidth responses as RDPEMT AutoDetectResponse
   sub-headers and sends them over `UdpTransport::send_message()`, including
   when `higher_layer_data` is empty. DVC bytes still take the existing
   Soft-Sync route; unsupported/malformed autodetect requests are ignored
   rather than causing a session abort.
4. Adds a debug marker when responses are sent and a diagnostic log on
   *main TCP/TLS read failure*, including whether UDP driver and pump tasks
   were still alive at that moment.
5. Adds a dedicated `ironrdp-client` integration-test target because
   `[lib] test = false` means inline library tests are not executed.

This closes a verified protocol gap. It **does not prove** that it is the sole
root cause of the observed TLS EOF. Do not mask an unresolved EOF by disabling
EGFX, dropping UDP packets, or claiming success from a configuration flag.

## Verification (manual; no automatic build or CI)

From this IronRDP branch on the Mac:

```bash
git switch fix/rdpeudp2-autodetect-eof-20261029
cargo test --locked -p ironrdp-client --features "rustls,udp" --test udp_autodetect
cargo test --locked -p ironrdp-rdpeudp-tokio -p ironrdp-rdpemt
cargo check --locked -p ironrdp-agent
cargo build --locked --release -p ironrdp-agent
```

**Use a separate experimental agent binary and endpoint.** Do not overwrite
Wendao's packaged binary or disconnect other owners' sessions. Confirm the
running process really uses the newly built SHA and starts after the build.

Run the existing four Wendao scenarios with the same 2560x1440 desktop and
fresh session per scenario. Start with UDP legacy to determine whether the
failure is independent of EGFX. Collect bounded `query-logs` immediately
after the connection, with the session log filter including
`ironrdp_client=debug,ironrdp_rdpeudp_tokio=debug,ironrdp_rdpeudp=debug`
when available. Watch specifically:

- `Sending RDPEMT UDP auto-detect responses on same tunnel` and its count.
- `Main TCP/TLS read failed; recording UDP task status before transport cleanup`.
- `RDP-UDP connection closed on error`, `Tunnel read failed, stopping read pump`,
  `UDP driver dropped without finishing`, their **relative order**.
- EGFX confirmation, UDP DVC activation and final RDP status.

Then verify **20+ seconds of active picture changes** (not a static desktop)
and the expected frame sequence, disconnect/reconnect cycles and a longer soak.
A missing marker on an idle/brief session is not proof that the server omitted
the request. If TCP EOF still occurs *before* the UDP driver exits, investigate
the exact fatal top-level reader error and Windows Disconnect/termination
message, not UDP keepalive intervals.

## Outstanding questions

- Was the TCP/TLS EOF the first failure or cleanup of a failed UDP tunnel?
- Does Windows actually issue RDPEMT auto-detect requests in the failing run?
- Did the Windows host send a Server Set Error Info PDU or an orderly disconnect?
- Do both UDP legacy and UDP EGFX still die once this fix is deployed?
- On the same server, can FreeRDP establish a lasting RDPEUDP2 tunnel, and
  what are the exact sequence/ACK/packet traces in a controlled comparison?

Only after this candidate survives live validation should it be considered
for merging back to `feat/wendao-udp-egfx-perf-20261028` or any production
branch.
