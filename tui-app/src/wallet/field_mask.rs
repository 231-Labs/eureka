use prost_types::FieldMask;
use sui_rpc::field::FieldMaskUtil;

/// Build a gRPC read mask（逗號分隔多條路徑；巢狀用 `.`）。
///
/// 注意：`ListDynamicFieldsRequest` 的 mask 是相對於 **`DynamicField`**（例如 `child_id`、`field_object.json`），
/// 不是 `ListDynamicFieldsResponse` 的 `dynamic_fields.*`——後者會被節點拒絕。
#[inline]
pub fn read_mask(paths: &str) -> FieldMask {
    <FieldMask as FieldMaskUtil>::from_str(paths)
}
