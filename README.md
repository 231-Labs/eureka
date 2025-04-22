# Eureka - 3D 列印智能合約系統

## 系統概述

Eureka 是一個基於 Sui 區塊鏈的 3D 列印服務智能合約系統，實現了列印機管理、任務分配和收益結算等功能。系統由智能合約和命令行應用兩部分組成。

## 技術架構

### 智能合約
- 部署環境：Sui devnet
- 合約地址：`0x1071e919f3260391059c17a7ada97c5ddb32751e1acb381cafa0742f9d5e08dd`
- 核心模組：
  - eureka：列印機管理
  - print_job：任務管理

### 命令行應用
- 開發語言：Rust
- 界面框架：Ratatui
- 運行時：Tokio
- 網絡支持：devnet/testnet/mainnet

## 系統功能

### 列印機管理
- 註冊：將列印機註冊到系統
- 狀態控制：在線/離線/忙碌
- 收益提取：自動結算和提取

### 任務管理
- 創建：新建列印任務
- 狀態追蹤：pending/printing/completed
- 進度監控：實時更新任務狀態

## 技術實現

### 智能合約
- 使用 Move 語言開發
- 實現共享對象管理
- 支持代幣交易

### 命令行應用
- 異步操作處理
- 多網絡環境支持
- 完整的錯誤處理

## 部署信息

### 合約地址
- Package ID: `0x1071e919f3260391059c17a7ada97c5ddb32751e1acb381cafa0742f9d5e08dd`
- PrinterRegistry: `0xfab040dbd9166fcf125110491490b899bb864b87ced83a3a3d4dfd2ddc650663`
- WAL Token: `0x356a26eb9e012a68958082340d4c4116e7f55615cf27affcff209cf0ae544f59::wal::WAL`

### 網絡配置
- Devnet: `https://fullnode.devnet.sui.io:443`
- Testnet: `https://fullnode.testnet.sui.io:443`
- Mainnet: `https://fullnode.mainnet.sui.io:443`

## 開發環境

### 系統需求
- Rust 1.70+
- Sui CLI
- WAL 代幣

### 依賴項
- sui-sdk
- ratatui
- tokio
- anyhow

## 更新日誌

### 2024-03-21
- 實現 PrinterCap 權限檢查
- 優化收益提取邏輯
- 改進錯誤處理機制

## 技術文檔

### 狀態定義
- 列印機狀態：
  - online: 可接受任務
  - busy: 執行中
  - offline: 不可用
- 任務狀態：
  - pending: 等待執行
  - printing: 執行中
  - completed: 已完成

### 交易流程
- 列印機註冊
- 任務創建
- 狀態更新
- 收益提取
