use prost_types::FieldMask;
use sui_rpc::field::FieldMaskUtil;

/// Build a gRPC read mask (comma-separated paths; use `.` for nesting).
///
/// Note: `ListDynamicFieldsRequest` masks are relative to **`DynamicField`** (e.g. `child_id`, `field_object.json`),
/// not `ListDynamicFieldsResponse`'s `dynamic_fields.*` — that form is rejected by the node.
#[inline]
pub fn read_mask(paths: &str) -> FieldMask {
    <FieldMask as FieldMaskUtil>::from_str(paths)
}
