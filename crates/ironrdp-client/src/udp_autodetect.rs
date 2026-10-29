//! UDP RDPEMT auto-detect subheader handling, separate from DRDYNVC payloads.
//!
//! The MS-RDPBCGR auto-detect header uses the same two bytes as the
//! RDPEMT subheader. The transport's recv() drops them; callers must use
//! recv_message() and send responses through the original UDP tunnel.

use ironrdp_core::{MonotonicInstant, decode, encode_vec};
use ironrdp_pdu::rdp::autodetect::AutoDetectRequest;
use ironrdp_rdpemt::{SubHeaderType, TunnelSubHeader};
use ironrdp_session::AutoDetectResponder;
use tracing::warn;

// The RDPEMT subheader is the complete MS-RDPBCGR auto-detect header:
// SubHeaderLength/Type are also autoDetectReq.headerLength/headerTypeId.
// Preserve it rather than using UdpTransport::recv(), which drops the
// subheaders and leaves the server's UDP RTT/bandwidth probes unanswered.
pub(crate) fn udp_autodetect_responses(
    sub_headers: &[TunnelSubHeader],
    responder: &mut AutoDetectResponder,
    now: MonotonicInstant,
) -> Vec<TunnelSubHeader> {
    let mut replies = Vec::new();
    for header in sub_headers {
        if header.sub_header_type != SubHeaderType::AutoDetectRequest {
            continue;
        }
        let Ok(length) = u8::try_from(header.wire_size()) else {
            warn!("Ignoring oversized RDPEMT auto-detect subheader");
            continue;
        };
        let mut encoded = Vec::with_capacity(header.wire_size());
        encoded.extend_from_slice(&[length, SubHeaderType::AutoDetectRequest.to_u8()]);
        encoded.extend_from_slice(&header.data);
        let request = match decode::<AutoDetectRequest>(&encoded) {
            Ok(request) => request,
            Err(error) => {
                warn!(%error, length = encoded.len(), "Ignoring invalid UDP auto-detect request");
                continue;
            }
        };
        let Some(response) = responder.respond(request, Some(now)) else {
            continue;
        };
        // AutoDetectResponse already encodes the same two-byte
        // headerLength/headerTypeId that RDPEMT uses for its subheader.
        let data = match encode_vec(&response) {
            Ok(data) => data,
            Err(error) => {
                warn!(%error, "Failed to encode UDP auto-detect response");
                continue;
            }
        };
        if data.len() < 2 || data[1] != SubHeaderType::AutoDetectResponse.to_u8() {
            warn!("Ignoring invalid encoded UDP auto-detect response");
            continue;
        }
        replies.push(TunnelSubHeader {
            sub_header_type: SubHeaderType::AutoDetectResponse,
            data: data[2..].to_vec(),
        });
    }
    replies
}

