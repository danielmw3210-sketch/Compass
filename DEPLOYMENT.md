# Compass Mainnet Deployment Guide

Complete guide for deploying Compass blockchain on Google Cloud Platform.

---

## Prerequisites

- **GCP Account** with billing enabled
- **VM specifications**: e2-medium or higher (2 vCPU, 4GB RAM minimum)
- **OS**: Ubuntu 22.04 LTS or Debian 12
- **Storage**: 50GB+ SSD

---

## Quick Start (Docker)

The fastest way to deploy:

```bash
# 1. SSH into your GCP VM
gcloud compute ssh your-vm-name

# 2. Install Docker
curl -fsSL https://get.docker.com | sh
sudo usermod -aG docker $USER
newgrp docker

# 3. Clone/upload your project
cd /opt
git clone <your-repo> compass && cd compass

# 4. Add your admin identity
cp /path/to/admin.json ./admin.json

# 5. Launch!
docker-compose up -d

# 6. Verify
curl http://localhost:9000/health
docker logs -f compass-node
```

---

## Manual Installation (Recommended for Validators)

### 1. Create GCP VM

```bash
gcloud compute instances create compass-mainnet \
  --zone=us-central1-a \
  --machine-type=e2-medium \
  --image-family=ubuntu-2204-lts \
  --image-project=ubuntu-os-cloud \
  --boot-disk-size=100GB \
  --boot-disk-type=pd-ssd \
  --tags=compass-node
```

### 2. Configure Firewall

```bash
# P2P port
gcloud compute firewall-rules create compass-p2p \
  --allow tcp:19000 \
  --target-tags compass-node \
  --description "Compass P2P"

# RPC port (optional - for public API)
gcloud compute firewall-rules create compass-rpc \
  --allow tcp:9000 \
  --target-tags compass-node \
  --description "Compass RPC"
```

### 3. Run Deployment Script

```bash
# SSH into VM
gcloud compute ssh compass-mainnet

# Upload and run deployment script
chmod +x scripts/deploy_mainnet.sh
sudo ./scripts/deploy_mainnet.sh
```

---

## Configuration Files

| File | Purpose | Location |
|------|---------|----------|
| `admin.json` | Node identity/signing key | `/opt/compass/admin.json` |
| `genesis.json` | Chain initialization | `/opt/compass/genesis.json` |
| `config.toml` | Node settings | `/opt/compass/config.toml` |

### Generate Admin Keys

```bash
./rust_compass keys generate --role admin --name admin
# Save the password securely!
```

---

## Service Management

```bash
# Status
sudo systemctl status compass-node

# Start/Stop/Restart
sudo systemctl start compass-node
sudo systemctl stop compass-node
sudo systemctl restart compass-node

# Logs
sudo tail -f /var/log/compass/node.log
sudo journalctl -u compass-node -f
```

---

## Ports Reference

| Port | Protocol | Purpose | Public? |
|------|----------|---------|---------|
| 19000 | TCP | P2P gossip/sync | ✅ Yes |
| 9000 | TCP | RPC/API | ⚠️ Optional |

---

## Monitoring

### Health Check
```bash
curl http://localhost:9000/health
```

### Node Info
```bash
curl http://localhost:9000/info | jq
```

### Block Height
```bash
curl http://localhost:9000/block/latest | jq '.height'
```

---

## Troubleshooting

### Node won't start
```bash
# Check logs
sudo journalctl -u compass-node -n 100

# Verify permissions
ls -la /opt/compass/
ls -la /var/lib/compass/
```

### Genesis mismatch
Ensure all nodes use identical `genesis.json`. The genesis hash must match:
```bash
./rust_compass genesis-hash
```

### Peer connection issues
- Verify firewall rules in GCP Console
- Check UFW: `sudo ufw status`
- Test port: `nc -zv <IP> 19000`

---

## Security Checklist

- [ ] Strong password for admin.json
- [ ] Firewall restricts RPC port (9000) if not needed publicly
- [ ] Regular system updates: `sudo apt update && sudo apt upgrade`
- [ ] Backup admin.json securely (offline)
- [ ] Enable GCP audit logging
