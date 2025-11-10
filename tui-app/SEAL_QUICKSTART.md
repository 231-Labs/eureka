# Seal 解密功能 - 快速開始

## 🚀 5 分鐘快速開始

### 步驟 1: 確認環境

```bash
# 確認 Sui 錢包已配置
sui client active-address

# 確認錢包配置文件存在
ls ~/.sui/sui_config/client.yaml
```

### 步驟 2: 編譯項目

```bash
cd /Users/harperdelaviga/eureka-1/tui-app

# 清理並重新編譯（如果遇到問題）
cargo clean
cargo build --release
```

### 步驟 3: 運行應用

```bash
cargo run --release
```

### 步驟 4: 測試解密功能

1. **選擇加密的 Sculpt**
   - 使用 ↑/↓ 鍵瀏覽列表
   - 查找帶有加密標記的模型

2. **開始列印**
   - 按 `p` 鍵
   - 觀察 Print Output 區域的日誌

3. **驗證解密**
   - 看到 "🔐 Encrypted model detected"
   - 看到 "✅ Model decrypted successfully"
   - 文件保存在 `Gcode-Transmit/test.stl`

## 🎯 日誌輸出示例

成功解密時的日誌：

```
[LOG] Downloading model from: https://aggregator.walrus-testnet.walrus.space/v1/abc123...
[LOG] 🔐 Encrypted model detected, attempting to decrypt...
[LOG] 🔐 Seal Resource ID: 0xabcd1234:sculptor_001
[LOG] 🔐 Initializing Seal decryption service...
[LOG] 🔐 Decrypting with package_id: 0xabcd1234
[LOG] 🔐 Resource ID: sculptor_001
[LOG] ✅ Model decrypted successfully
[LOG] 3D model downloaded successfully
```

## ❌ 常見錯誤

### 錯誤 1: 權限被拒絕

```
[LOG] ❌ Decryption failed: permission denied
```

**解決**: 確保你的地址在 Sculpt 白名單中

### 錯誤 2: Session Key 創建失敗

```
[LOG] ❌ Failed to create session key
```

**解決**: 檢查錢包配置和網絡連接

### 錯誤 3: 下載失敗

```
[LOG] ❌ Failed to download 3D model
```

**解決**: 檢查 Walrus 網絡狀態

## 🧪 測試加密功能

### 在 Archimeters 前端創建加密 Sculpt

1. 訪問 Archimeters 前端
2. Mint 新的 Sculpt
3. ✅ 勾選 "Generate STL" 選項
4. ✅ 勾選 "Encrypt STL" 選項
5. 添加你的列印機地址到白名單
6. 在 Eureka TUI 中測試解密

## 📋 檢查清單

在開始前確認：

- [ ] Sui 錢包已配置
- [ ] 錢包有足夠的 SUI 和 WAL
- [ ] 網絡連接正常
- [ ] 你的地址在目標 Sculpt 的白名單中
- [ ] Eureka TUI 已編譯成功

## 🔗 更多資源

- 詳細文檔: `SEAL_DECRYPTION.md`
- 變更日誌: `CHANGELOG_SEAL.md`
- 測試示例: `examples/seal_decryption_test.rs`

## 💡 提示

- 解密過程完全自動，無需手動操作
- Session Key 有效期 5 分鐘
- 未加密的 Sculpt 繼續正常工作
- 所有操作都有詳細日誌

---

**開始使用吧！** 🎉

