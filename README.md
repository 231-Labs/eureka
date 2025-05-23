# 🛠️ Eureka - 3D Printing TUI App Build on Sui

***Physical Manufacturing Layer for the Archimeters Ecosystem*** 🔄

| Offline Mode                                                                                | Online Mode                                                                                 |
|---------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------|
| ![offline_mode](https://github.com/user-attachments/assets/0eacc6dd-07d2-4635-914a-b536e90ad202) | ![shotEasy](https://github.com/user-attachments/assets/f2cc36a6-6f04-4caa-af0c-015814305e10) |


## 📋 Project Overview
Eureka is a TUI application for 3D printing based on the Sui blockchain. It serves as a companion system for the [Archimeters](https://github.com/231-Labs/archimeters) parametric design platform, handling the transformation of digital designs into physical products. Currently in the prototype stage, it has implemented basic printer registration, task assignment, and revenue settlement (in development) functionalities. The long-term goal is to create a global distributed manufacturing network by increasing the number of users.

## 💻 Technical Implementation

### Hardware Platform
- Designed to run on Raspberry Pi 5 + Ubuntu environment
- Low resource consumption for stable operation
- We aim to keep Eureka's hardware requirements accessible

### Smart Contracts (Move)
- **PrinterRegistry**: Manages 3D printer registration and status
- **PrintJob**: Handles print task creation and execution
- **DesignIntegration**: Interface for interoperability with Archimeters design assets

### Terminal Application (Rust)
- TUI interface built with Ratatui
- Blockchain interaction via sui-sdk

### Why Choose a TUI Application?
- Hardware-friendly: Works well on resource-constrained devices (like Raspberry Pi) without requiring additional graphical processing resources
- Low barrier to entry: Enables more people to join the manufacturing network at a lower cost
- Remote operation: Facilitates remote monitoring and management via SSH

## 🔄 Integration with Archimeters

Eureka and Archimeters form a design-to-manufacturing ecosystem, enabling transition from digital creativity to physical products:

### Design → Manufacturing Bridge 🌉
- **On-chain Asset Access**: Direct access to design files and parameters stored in Walrus
- **Dual Working Modes**: 
  - Offline mode for printing 3D models from the user's wallet
  - Online mode for receiving commissioned tasks from the Archimeters platform

### Manufacturing Layer 🏭
- **Automated Workflow**: One-click startup simplifies the process from accepting orders to completion
- **Status Synchronization**: Print progress is recorded on-chain for transparency
- **G-code Converter**: Converts design parameters into executable instructions for the printer

### Economic Incentive System (In Development) 💸
- **Revenue Settlement**: Manufacturing revenue is distributed upon task completion
- **On-demand Manufacturing**: Transforms designs into physical products, reducing resource waste
- **Future Manufacturing Network**: Plans to connect designers and printing resource providers globally

## 🧪 Development Status

The project is currently in the experimental prototype stage as a TUI application. Implemented features include:
- Basic contract structure and printer registration
- TUI application framework with blockchain interaction
- On-chain printing task status

Under development:
- Commissioned printing task payment mechanism
- Integration of Seal decryption functionality for NFT access control

Future goals:
- Scaling to a global distributed manufacturing network through user adoption

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
