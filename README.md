# Eureka 3D 列印智能合約

## 部署資訊

### 環境
- 網路：Sui devnet

### 合約地址
- Package ID: `0x0f954776ba10e542e9a8ae9bf617aa4e3af99d800c1fefdf24dd6d55c0dfcf58`
- 模組：
  - eureka
  - print_job

### 重要物件
- PrinterRegisty (Shared): `0x68cd9ef41a500fb8d35dbc2dccffb5594e68596516ae27f72100cba3dc7e9781`
- UpgradeCap: `0x91a6547ec0a91069cf5cd5d4f8cb800158304e8ac4a2cef4d105b46d803999c5`

### 更新記錄
- 2024-03-21: 修復權限檢查漏洞
  - 添加 PrinterCap 的權限檢查邏輯
  - 確保只有正確的 PrinterCap 持有者可以更新打印機狀態和提取收益

## 合約功能

### Printer 管理
- 註冊新的列印機（默認離線狀態）
- 更新列印機狀態（離線/在線）
- 提取列印機收益

### Print Job 管理
- 創建列印任務
- 更新列印任務狀態
- 追蹤列印進度

## 狀態定義

### Printer 狀態
- `online`: 列印機在線，可接受新任務
- `busy`: 列印機正在執行任務
- `offline`: 列印機離線

### Print Job 狀態
- `pending`: 等待開始
- `printing`: 正在列印
- `completed`: 列印完成
