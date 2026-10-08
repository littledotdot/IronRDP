# IronRDP RDP-UDP

Reliable UDP transport implemented as described in [MS-RDPEUDP] and
[MS-RDPEUDP2].

The two documents divide the work. [MS-RDPEUDP] defines the handshake that
opens a connection and negotiates a protocol version; from version 3 onward
that handshake leads into the data transfer defined by [MS-RDPEUDP2], which is
the one this crate implements. Note that the documents take opposite byte
orders, and number the bits in their diagrams in opposite directions.

Sans-I/O: the state machine is driven by datagrams and by a caller-supplied
instant, and returns the datagrams it wants sent. It performs no I/O and reads
no clock, so it can be driven by any runtime.

For Windows RDP-UDP2 interoperability, channel sequence numbers start at one
and skip zero on each 16-bit wrap, as specified in [MS-RDPEUDP2 Appendix A,
note 1][windows-channel-sequence]. Data sequence numbers still wrap through
zero. The legacy RDP-UDP version 1/2 sequence space is unchanged. The default
window is 32,768 packets, the bounded RDP-UDP2 maximum, to accommodate Windows
graphics bursts and retransmission recovery.

This crate is part of the [IronRDP] project.

[IronRDP]: https://github.com/Devolutions/IronRDP
[MS-RDPEUDP]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpeudp/
[MS-RDPEUDP2]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpeudp2/

[windows-channel-sequence]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rdpeudp2/add0cb95-3df2-45ce-8b6b-bb18b6f03fbe
