#!/bin/bash
# =============================================================================
# Compass Mainnet Deployment Script for Google Cloud VM (Ubuntu/Debian)
# =============================================================================
set -e

echo "🚀 Compass Mainnet Deployment Script"
echo "======================================"

# Configuration
COMPASS_USER="compass"
COMPASS_DIR="/opt/compass"
DATA_DIR="/var/lib/compass"
LOG_DIR="/var/log/compass"
P2P_PORT=19000
RPC_PORT=9000

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# =============================================================================
# 1. System Dependencies
# =============================================================================
install_dependencies() {
    log_info "Installing system dependencies..."
    sudo apt-get update
    sudo apt-get install -y \
        build-essential \
        pkg-config \
        libssl-dev \
        libclang-dev \
        cmake \
        git \
        curl \
        htop \
        jq
}

# =============================================================================
# 2. Install Rust
# =============================================================================
install_rust() {
    if command -v rustc &> /dev/null; then
        log_info "Rust already installed: $(rustc --version)"
    else
        log_info "Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
}

# =============================================================================
# 3. Create Compass User and Directories
# =============================================================================
setup_directories() {
    log_info "Setting up directories..."
    
    # Create compass user if it doesn't exist
    if ! id "$COMPASS_USER" &>/dev/null; then
        sudo useradd -r -s /bin/false -d $COMPASS_DIR $COMPASS_USER
    fi
    
    # Create directories
    sudo mkdir -p $COMPASS_DIR $DATA_DIR $LOG_DIR
    sudo chown -R $COMPASS_USER:$COMPASS_USER $COMPASS_DIR $DATA_DIR $LOG_DIR
}

# =============================================================================
# 4. Build from Source
# =============================================================================
build_compass() {
    log_info "Building Compass from source (release mode)..."
    
    # Clone or update repository
    if [ -d "$COMPASS_DIR/src" ]; then
        cd $COMPASS_DIR/src
        git pull
    else
        sudo mkdir -p $COMPASS_DIR/src
        sudo chown $USER:$USER $COMPASS_DIR/src
        # Replace with your actual repo URL
        # git clone https://github.com/your-org/rust_compass.git $COMPASS_DIR/src
        log_warn "No git repo configured. Copy source manually to $COMPASS_DIR/src"
        exit 1
    fi
    
    cd $COMPASS_DIR/src
    source "$HOME/.cargo/env"
    cargo build --release
    
    # Copy binary
    sudo cp target/release/rust_compass $COMPASS_DIR/
    sudo chown $COMPASS_USER:$COMPASS_USER $COMPASS_DIR/rust_compass
    sudo chmod +x $COMPASS_DIR/rust_compass
}

# =============================================================================
# 5. Install Binary (from pre-built - alternative to building)
# =============================================================================
install_binary() {
    if [ -f "./rust_compass" ]; then
        log_info "Installing pre-built binary..."
        sudo cp ./rust_compass $COMPASS_DIR/
        sudo chown $COMPASS_USER:$COMPASS_USER $COMPASS_DIR/rust_compass
        sudo chmod +x $COMPASS_DIR/rust_compass
    else
        log_error "No rust_compass binary found. Build from source or provide binary."
        exit 1
    fi
}

# =============================================================================
# 6. Configure Node
# =============================================================================
configure_node() {
    log_info "Creating node configuration..."
    
    sudo tee $COMPASS_DIR/config.toml > /dev/null <<EOF
[node]
p2p_port = $P2P_PORT
rpc_port = $RPC_PORT
db_path = "$DATA_DIR/mainnet.db"
log_level = "info"
identity_file = "$COMPASS_DIR/admin.json"

[consensus]
slot_duration_ms = 1000
EOF

    sudo chown $COMPASS_USER:$COMPASS_USER $COMPASS_DIR/config.toml
    log_info "Configuration saved to $COMPASS_DIR/config.toml"
}

# =============================================================================
# 7. Install Systemd Service
# =============================================================================
install_service() {
    log_info "Installing systemd service..."
    
    sudo tee /etc/systemd/system/compass-node.service > /dev/null <<EOF
[Unit]
Description=Compass Blockchain Node
After=network.target
Wants=network-online.target

[Service]
Type=simple
User=$COMPASS_USER
Group=$COMPASS_USER
WorkingDirectory=$COMPASS_DIR
ExecStart=$COMPASS_DIR/rust_compass node start --config $COMPASS_DIR/config.toml
Restart=always
RestartSec=10
LimitNOFILE=65535

# Logging
StandardOutput=append:$LOG_DIR/node.log
StandardError=append:$LOG_DIR/node-error.log

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$DATA_DIR $LOG_DIR

[Install]
WantedBy=multi-user.target
EOF

    sudo systemctl daemon-reload
    sudo systemctl enable compass-node
    log_info "Service installed and enabled"
}

# =============================================================================
# 8. Configure Firewall
# =============================================================================
configure_firewall() {
    log_info "Configuring firewall rules..."
    
    if command -v ufw &> /dev/null; then
        sudo ufw allow $P2P_PORT/tcp comment 'Compass P2P'
        sudo ufw allow $RPC_PORT/tcp comment 'Compass RPC'
        log_info "UFW rules added for ports $P2P_PORT and $RPC_PORT"
    else
        log_warn "UFW not found. Remember to open ports $P2P_PORT and $RPC_PORT in GCP Console!"
    fi
}

# =============================================================================
# 9. Setup Log Rotation
# =============================================================================
setup_logrotate() {
    log_info "Setting up log rotation..."
    
    sudo tee /etc/logrotate.d/compass > /dev/null <<EOF
$LOG_DIR/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0640 $COMPASS_USER $COMPASS_USER
    postrotate
        systemctl kill -s HUP compass-node.service 2>/dev/null || true
    endscript
}
EOF
}

# =============================================================================
# 10. Start Node
# =============================================================================
start_node() {
    log_info "Starting Compass node..."
    sudo systemctl start compass-node
    sleep 3
    
    if systemctl is-active --quiet compass-node; then
        log_info "✅ Compass node is running!"
        log_info "   Check status: sudo systemctl status compass-node"
        log_info "   View logs:    sudo tail -f $LOG_DIR/node.log"
        log_info "   RPC endpoint: http://localhost:$RPC_PORT"
    else
        log_error "Failed to start node. Check logs: sudo journalctl -u compass-node -n 50"
        exit 1
    fi
}

# =============================================================================
# Main
# =============================================================================
main() {
    echo ""
    echo "Select installation mode:"
    echo "  1) Full install (build from source)"
    echo "  2) Binary install (use pre-built rust_compass)"
    echo "  3) Service only (configure systemd for existing binary)"
    echo ""
    read -p "Choice [1-3]: " choice
    
    case $choice in
        1)
            install_dependencies
            install_rust
            setup_directories
            build_compass
            configure_node
            install_service
            configure_firewall
            setup_logrotate
            start_node
            ;;
        2)
            install_dependencies
            setup_directories
            install_binary
            configure_node
            install_service
            configure_firewall
            setup_logrotate
            start_node
            ;;
        3)
            configure_node
            install_service
            configure_firewall
            setup_logrotate
            log_info "Service configured. Start with: sudo systemctl start compass-node"
            ;;
        *)
            log_error "Invalid choice"
            exit 1
            ;;
    esac
    
    echo ""
    log_info "🎉 Deployment complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Copy your admin.json to $COMPASS_DIR/admin.json"
    echo "  2. Copy genesis.json to $COMPASS_DIR/genesis.json"
    echo "  3. Open ports in GCP Console: $P2P_PORT (P2P), $RPC_PORT (RPC)"
    echo "  4. Restart node: sudo systemctl restart compass-node"
}

main "$@"
