# 🛠️ Eureka - 3D Printing TUI App Build on Sui

***Physical Manufacturing Layer for the Archimeters Ecosystem*** 🔄

| Offline Mode                                                                                      | Online Mode                                                                                  |
| ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| ![offline\_mode](https://github.com/user-attachments/assets/0eacc6dd-07d2-4635-914a-b536e90ad202) | ![shotEasy](https://github.com/user-attachments/assets/f2cc36a6-6f04-4caa-af0c-015814305e10) |

## 📋 Project Overview

Eureka is a TUI application for 3D printing based on the Sui blockchain. It serves as a companion system for the [Archimeters](https://github.com/231-Labs/archimeters) parametric design platform, handling the transformation of digital designs into physical products. Currently in the prototype stage, it has implemented basic printer registration, task assignment, and revenue settlement (in development) functionalities. The long-term goal is to create a global distributed manufacturing network by increasing the number of users.

## 💻 Technical Implementation

### Hardware Platform

* Designed to run on Raspberry Pi 5 + Ubuntu environment
* Low resource consumption for stable operation
* We aim to keep Eureka's hardware requirements accessible

### 📌 Assigning a Persistent USB Name for the 3D Printer

To ensure stable and consistent access to your 3D printer's serial port on Raspberry Pi or Ubuntu systems, you can assign a persistent device name using a custom `udev` rule. This is especially useful when the default `/dev/ttyUSB0` can change across reboots or when multiple USB devices are connected.

#### Why This Matters

USB serial devices like CH340 or FTDI may receive different names (`/dev/ttyUSB0`, `/dev/ttyUSB1`, etc.) depending on the order they are plugged in. This can cause issues for scripts or systems expecting a specific device path. By assigning a static symlink (e.g., `/dev/3Dprinter`), you ensure the device is always accessible using the same name.

#### 📘 Step-by-Step Setup

```bash
# 1. Identify your USB device
lsusb
# Example output:
# Bus 001 Device 005: ID 1a86:7523 QinHeng Electronics CH340 serial converter
# Note the idVendor and idProduct values

# 2. Create a udev rule
sudo nano /etc/udev/rules.d/99-usb-serial.rules

# 3. Add the following rule (replace idVendor/idProduct if different)
SUBSYSTEM=="tty", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523", SYMLINK+="3Dprinter"

# Save and exit (Ctrl+O, then Ctrl+X)

# 4. Reload the rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# 5. Replug the USB device

# 6. Verify that the new symlink exists
ls -l /dev/3Dprinter
```

If successful, you will see `/dev/3Dprinter` as a symbolic link pointing to `/dev/ttyUSBx`.

> ✅ This ensures your TUI app can reliably connect to the correct device regardless of USB assignment order.

### Smart Contracts (Move)

* **PrinterRegistry**: Manages 3D printer registration and status
* **PrintJob**: Handles print task creation and execution
* **DesignIntegration**: Interface for interoperability with Archimeters design assets

### Terminal Application (Rust)

* TUI interface built with Ratatui
* Blockchain interaction via sui-sdk

### Why Choose a TUI Application?

* Hardware-friendly: Works well on resource-constrained devices (like Raspberry Pi) without requiring additional graphical processing resources
* Low barrier to entry: Enables more people to join the manufacturing network at a lower cost
* Remote operation: Facilitates remote monitoring and management via SSH

## 🔄 Integration with Archimeters

Eureka and Archimeters form a design-to-manufacturing ecosystem, enabling transition from digital creativity to physical products:

### Design → Manufacturing Bridge 🌉

* **On-chain Asset Access**: Direct access to design files and parameters stored in Walrus
* **Dual Working Modes**:

  * Offline mode for printing 3D models from the user's wallet
  * Online mode for receiving commissioned tasks from the Archimeters platform

### Manufacturing Layer 🏠

* **Automated Workflow**: One-click startup simplifies the process from accepting orders to completion
* **Status Synchronization**: Print progress is recorded on-chain for transparency
* **G-code Converter**: Converts design parameters into executable instructions for the printer

### Economic Incentive System (In Development) 💸

* **Revenue Settlement**: Manufacturing revenue is distributed upon task completion
* **On-demand Manufacturing**: Transforms designs into physical products, reducing resource waste
* **Future Manufacturing Network**: Plans to connect designers and printing resource providers globally

## 🧪 Development Status

The project is currently in the experimental prototype stage as a TUI application. Implemented features include:

* Basic contract structure and printer registration
* TUI application framework with blockchain interaction
* On-chain printing task status

Under development:

* Commissioned printing task payment mechanism
* Integration of Seal decryption functionality for NFT access control

Future goals:

* Scaling to a global distributed manufacturing network through user adoption

## 🚀 Usage

### Requirements

* Rust 1.70+
* Sui CLI

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

* Devnet: `https://fullnode.devnet.sui.io:443`
* Testnet: `https://fullnode.testnet.sui.io:443`
* Mainnet: `https://fullnode.mainnet.sui.io:443`

---

*Eureka is an experimental project developed during a hackathon, working together with Archimeters to build a decentralized solution that bridges digital design with physical manufacturing.* 🔬
