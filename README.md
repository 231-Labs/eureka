# 🛠️ Eureka - 3D Printing TUI App Build on Sui

<div align="center">

## Physical Manufacturing Layer for the Archimeters Ecosystem 🔄

| Offline Mode                                                                                | Online Mode                                                                                 |
|---------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------|
| ![Image 1](https://github.com/user-attachments/assets/53089412-ebde-4ce5-a943-7ea894c10352) | ![Image 2](https://github.com/user-attachments/assets/da9837f7-9863-4900-9f82-d5113ab6ee39) |


## 📋 Project Overview

Eureka is an experimental decentralized 3D printing manufacturing network based on the Sui blockchain. It serves as a companion system for the [Archimeters](https://github.com/231-Labs/archimeters) parametric design platform, responsible for transforming digital designs into physical products. Currently in the prototype stage, it has implemented basic printer registration, task assignment, and revenue settlement (in development) functionalities.

## 💻 Technical Implementation

### Hardware Platform
- Optimized to run on Raspberry Pi 5 + Ubuntu environment
- Low resource consumption, suitable for long-term stable operation
- We aim to keep Eureka's hardware requirements accessible, allowing more users to experience distributed manufacturing

### Smart Contracts (Move)
- **PrinterRegistry**: Manages 3D printer registration and status
- **PrintJob**: Handles print task creation and execution
- **DesignIntegration**: Interface for interoperability with Archimeters design assets

### Terminal Application (Rust)
- TUI interface built with Ratatui
- Blockchain interaction via sui-sdk

### Why Choose a TUI Application?
- Hardware-friendly: Performs excellently on resource-constrained devices (like Raspberry Pi) without requiring additional graphical processing resources
- Cost-effective: Lowers the barrier to entry, enabling more people to join the manufacturing network at a low cost
- Remote operation: Facilitates remote monitoring and management via SSH

## 🔄 Integration with Archimeters

Eureka and Archimeters form a complete design-to-manufacturing ecosystem, enabling seamless transition from digital creativity to physical products:

### Design → Manufacturing Bridge 🌉
- **On-chain Asset Access**: Direct access to design files and parameters stored in Walrus
- **Dual Working Modes**: 
  - Offline mode for printing 3D models from the user's wallet
  - Online mode for receiving commissioned tasks from the Archimeters platform

### Intelligent Manufacturing Layer 🏭
- **Automated Workflow**: One-click startup feature simplifies the entire process from accepting orders to completion
- **Real-time Status Synchronization**: Print progress is recorded on-chain in real-time, ensuring transparency
- **G-code Converter**: Automatically converts design parameters into executable instructions for the printer

### Economic Incentive System (In Development) 💸
- **Instant Settlement**: Manufacturing revenue is automatically distributed upon task completion
- **On-demand Manufacturing**: Transforms designs into physical products instantly, reducing resource waste
- **Decentralized Marketplace**: Connects designers and printing resource providers globally

## 🧪 Development Status

The project is currently in the experimental prototype stage. Implemented features include:
- Basic contract structure and printer registration
- TUI application framework with blockchain interaction
- On-chain printing task status

Under development:
- Commissioned printing task payment mechanism
- Integration of Seal decryption functionality for complete NFT access control

## 🚀 Usage

### Requirements
- Rust 1.70+
- Sui CLI

### Installation Steps
```bash
# Clone the repository
git clone https://github.com/231-Labs/eureka.git

# Compile the application
cd eureka/tui-app
cargo build

# Run the application
cargo run
```

### Configuration
The application supports the following network configurations:
- Devnet: `https://fullnode.devnet.sui.io:443`
- Testnet: `https://fullnode.testnet.sui.io:443`
- Mainnet: `https://fullnode.mainnet.sui.io:443`

---

*Eureka is an experimental project developed during a hackathon, working together with Archimeters to build a decentralized solution that bridges digital design with physical manufacturing.* 🔬
