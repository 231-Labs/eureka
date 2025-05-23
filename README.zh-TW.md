# 🛠️ Eureka - 3D 列印智能合約系統

> Archimeters 生態系統的實體製造層組件 🔄


## 📋 專案概述

Eureka 是基於 Sui 區塊鏈的 3D 列印 TUI 應用程式，作為 [Archimeters](https://github.com/231-Labs/archimeters) 參數化設計平台的配套系統，負責將數位設計轉化為實體產品。目前處於原型階段，實現了基礎的列印機註冊、任務分配和收益結算(開發中)功能。長期目標是透過增加用戶數來創建全球分散式製造網絡。

## 💻 技術實現

### 硬體平台
- 設計為在 Raspberry Pi 5 + Ubuntu 環境下運行
- 低資源消耗，適合穩定運行
- 我們希望 Eureka 的硬體配置要求足夠容易入門

### 智能合約 (Move)
- **PrinterRegistry**: 管理 3D 列印機註冊和狀態
- **PrintJob**: 處理列印任務創建和執行
- **DesignIntegration**: 與 Archimeters 設計資產互操作接口

### 終端應用 (Rust)
- 基於 Ratatui 構建的 TUI 界面
- sui-sdk 與區塊鏈交互

### 為何選擇 TUI 應用？
- 硬體友好: 在資源受限的設備 (如 Raspberry Pi) 上運行良好，無需額外圖形處理資源
- 低參與門檻: 使更多人能夠以較低成本加入製造網絡
- 遠端操作: 便於通過 SSH 進行遠程監控和管理

## 🔄 與 Archimeters 整合

Eureka 與 Archimeters 構成設計-製造生態系統，實現從數位創意到實體產品的轉換：

### 設計 → 製造橋接 🌉
- **鏈上資產讀取**: 直接訪問 Walrus 中存儲的設計文件和參數
- **雙向工作模式**: 
  - 離線模式下直接列印用戶錢包中的 3D 模型
  - 線上模式接收來自 Archimeters 平台的委託任務

### 製造層 🏭
- **自動化工作流**: 一鍵啟動功能簡化從接單到完成的流程
- **狀態同步**: 列印過程中的狀態上鏈，確保透明度
- **G-code 轉換器**: 將設計參數轉換為設備可執行的指令

### 經濟激勵系統(開發中) 💸
- **收益結算**: 任務完成後製造收益分配
- **按需製造**: 將設計轉化為實體產品，減少資源浪費
- **未來製造網絡**: 計劃連接全球設計師與列印資源提供者

## 🧪 開發狀態

目前專案處於實驗原型階段的 TUI 應用程式。已實現功能:
- 基礎合約結構與列印機註冊
- TUI 應用框架與區塊鏈交互
- 鏈上列印任務狀態

正在開發:
- 委託列印任務付費機制
- 整合 Seal 解密功能，實現 NFT 存取權控制

未來目標:
- 透過用戶採用擴展成全球分散式製造網絡

## 🚀 使用方法

### 環境需求
- Rust 1.70+
- Sui CLI

### 安裝步驟
```bash
# 克隆代碼庫
git clone https://github.com/231-Labs/eureka.git

# 編譯應用
cd eureka/tui-app
cargo build

# 運行應用
cargo run
```

### 配置說明
應用支持以下網絡配置:
- Devnet: `https://fullnode.devnet.sui.io:443`
- Testnet: `https://fullnode.testnet.sui.io:443`
- Mainnet: `https://fullnode.mainnet.sui.io:443`

---

*Eureka 是黑客松期間開發的實驗性項目，與 Archimeters 共同構建數位設計到實體製造的去中心化解決方案。* 🔬 