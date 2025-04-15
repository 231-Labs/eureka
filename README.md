# Eureka 3D 列印智能合約

## 部署資訊

### 環境
- 網路：Sui devnet

### 合約地址
- Package ID: `0x30e757c78a72a8b2cf62a61cf38bfc5e19c9cbaee028af2d0e6b7d7bd809e547`
- 模組：
  - eureka
  - print_job

### 重要物件
- PrinterRegisty (Shared): `0xf2147e5d16420e6f3bef05b00177172271a7e735877515ca35f623b3dd612c27`
- UpgradeCap: `0x07ed084e63a86768c3428daaf27aa7a9baeadccde6edb2f75784ed829be93bd0`

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
