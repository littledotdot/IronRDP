# IronRDP Daemon

Reusable persistent RDP-session support for local RPC hosts. It owns the daemon lifecycle,
retained framebuffer, input and screenshot handling, session log buffer, NOW endpoint, and durable
NOW operation manager. The local RPC schema and transport are provided directly by
[`ironrdp-rpc`](../ironrdp-rpc).

## Windows RDPDR options

`DaemonOptions` can configure fixed redirected volumes (`with_rdpdr_drives`) and WinSCard smartcard
redirection (`with_smartcard`).

Smartcard is also honored from connect/overlay property `ironrdp_smartcard`:

- startup `--smartcard` / `with_smartcard(true)` / overlay `ironrdp_smartcard:i:1` sets the default
- connect-time `ironrdp_smartcard:i:1` can enable smartcard-only without a startup flag
- connect-time `ironrdp_smartcard:i:0` disables smartcard for that session

Smartcard-only sessions use an empty drive list with device ID `0` reserved for the smartcard device.

## Wendao transport and frame stream

This integration build includes the client `udp` feature. Set connect/overlay property
`ironrdp_udp_transport:i:1` to request reliable UDP multitransport, or `:0` for TCP.
Unsupported transports and failed bootstrap keep TCP; migrated-channel loss reconnects.
FrameStream uses request tag 41 and BGR24 payload tag 18, with one-byte credits.
Update Wendao and the agent together, then restart the daemon.
