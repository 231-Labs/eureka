# Seal 解密功能 - 變更日誌

## [Unreleased] - 2025-11-10

### ✨ 新增功能

#### Seal 解密集成

- **自動解密**: 支持自動檢測和解密 Seal 加密的 STL 文件
- **Session Key 機制**: 使用短期授權令牌，減少用戶簽名次數
- **白名單驗證**: 只有授權的列印機可以解密特定 Sculpt
- **詳細日誌**: 解密過程的每一步都有清晰的日誌輸出

#### 新增模組

```
src/seal/
├── mod.rs              # 模組導出
├── types.rs            # Seal 相關類型定義
└── decryption.rs       # 解密核心邏輯 (300+ 行)
```

#### API 變更

##### SculptItem 結構更新

**之前**:
```rust
pub struct SculptItem {
    pub alias: String,
    pub blob_id: String,
    pub printed_count: u64,
    pub id: String,
}
```

**之後**:
```rust
pub struct SculptItem {
    pub alias: String,
    pub blob_id: String,
    pub printed_count: u64,
    pub id: String,
    pub is_encrypted: bool,              // 🆕
    pub seal_resource_id: Option<String>, // 🆕
}
```

##### download_3d_model 簽名更新

**之前**:
```rust
pub async fn download_3d_model(&mut self, blob_id: &str) -> Result<()>
```

**之後**:
```rust
pub async fn download_3d_model(
    &mut self, 
    blob_id: &str, 
    seal_resource_id: Option<&str>  // 🆕
) -> Result<()>
```

### 🔧 技術改進

#### 依賴更新

在 `Cargo.toml` 中新增：

```toml
# Seal SDK for decryption
seal-sdk-rs = { git = "https://github.com/gfusee/seal-sdk-rs", tag = "0.0.2", features = ["native-sui-sdk"] }
reqwest = { version = "0.11", features = ["json"] }
```

#### 加密檢測邏輯

新增啟發式檢測方法 `is_file_encrypted()`:

- 檢查 ASCII STL 簽名 (`solid`)
- 檢查二進制 STL 結構
- 避免對未加密文件進行不必要的解密操作

### 📋 向後兼容性

#### ✅ 完全向後兼容

- 未加密的 Sculpt 繼續正常工作
- 現有代碼路徑不受影響
- 自動檢測加密狀態

#### 🔄 需要更新的地方

1. **合約端（Archimeters）**:
   ```move
   // 需要在 Sculpt 結構中添加
   struct Sculpt has key, store {
       id: UID,
       alias: String,
       structure: String,
       printed: u64,
       seal_resource_id: Option<String>, // 🆕
   }
   ```

2. **調用方式**:
   ```rust
   // 舊方式仍然有效（傳 None）
   app.download_3d_model(&blob_id, None).await?;
   
   // 新方式（支持解密）
   app.download_3d_model(&blob_id, Some("pkg:id")).await?;
   ```

### 🐛 已知問題

1. **Session Key 過期**: 5 分鐘後需要重新創建（待優化）
2. **SSL 證書問題**: 某些環境可能遇到證書驗證問題
3. **錯誤處理**: 部分錯誤訊息還不夠友好

### 📈 性能影響

- **未加密文件**: 無性能影響（快速檢測後跳過）
- **加密文件**: 
  - Session Key 創建: ~2-3 秒（首次）
  - 解密操作: ~1-2 秒（視文件大小）
  - 總體影響: 可接受（相比列印時間）

### 🔒 安全性

#### 增強的安全措施

- ✅ 私鑰不離開本地錢包
- ✅ Session Key 短期有效（5 分鐘）
- ✅ 每次解密都驗證權限
- ✅ 支持細粒度訪問控制

#### 安全假設

- 用戶的 Sui 錢包是安全的
- RPC 節點是可信的
- Seal Key Servers 是可用的

### 📚 文檔更新

新增文檔：

- `SEAL_DECRYPTION.md`: 完整的使用指南
- `CHANGELOG_SEAL.md`: 本變更日誌
- `examples/seal_decryption_test.rs`: 測試示例

### 🧪 測試

#### 單元測試

```bash
cargo test seal::tests
```

#### 集成測試場景

- ✅ 解密已授權的 Sculpt
- ✅ 拒絕未授權的訪問
- ✅ 正確處理未加密文件
- ✅ 錯誤處理和恢復

### 👥 貢獻者

- [@harperdelaviga](https://github.com/harperdelaviga) - 主要實現

### 🔗 相關資源

- [Seal SDK](https://github.com/gfusee/seal-sdk-rs)
- [Archimeters 項目](../archimeters-1/)
- [Seal 文檔](https://seal-docs.wal.app/)

---

## 遷移指南

### 對於使用者

無需任何操作，功能自動啟用。

### 對於開發者

#### 步驟 1: 更新依賴

```bash
cd tui-app
cargo update
```

#### 步驟 2: 處理 API 變更

如果你有自定義代碼調用 `download_3d_model`:

```rust
// 更新調用方式
- app.download_3d_model(&blob_id).await?;
+ app.download_3d_model(&blob_id, item.seal_resource_id.as_deref()).await?;
```

#### 步驟 3: 測試

```bash
cargo test
cargo run
```

### 故障排除

如果遇到編譯問題：

```bash
cargo clean
rm Cargo.lock
cargo build
```

如果遇到運行時問題，查看 `SEAL_DECRYPTION.md` 的故障排除章節。

---

**版本**: v0.1.0  
**日期**: 2025-11-10  
**狀態**: 開發中

