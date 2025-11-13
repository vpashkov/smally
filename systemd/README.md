# Systemd Service Files

These systemd service files allow you to manage Smally API as a system service.

## Installation

### 1. Copy Service Files

```bash
sudo cp systemd/smally.service /etc/systemd/system/
sudo cp systemd/smally-backup.service /etc/systemd/system/
sudo cp systemd/smally-backup.timer /etc/systemd/system/
```

### 2. Reload Systemd

```bash
sudo systemctl daemon-reload
```

### 3. Enable Services

```bash
# Enable Smally API to start on boot
sudo systemctl enable smally.service

# Enable automatic daily backups
sudo systemctl enable smally-backup.timer
```

### 4. Start Services

```bash
# Start Smally API
sudo systemctl start smally.service

# Start backup timer
sudo systemctl start smally-backup.timer
```

## Usage

### Check Status

```bash
# Check API service status
sudo systemctl status smally.service

# Check backup timer status
sudo systemctl status smally-backup.timer

# List all timers
sudo systemctl list-timers
```

### Start/Stop/Restart

```bash
# Start
sudo systemctl start smally.service

# Stop
sudo systemctl stop smally.service

# Restart
sudo systemctl restart smally.service

# Reload configuration
sudo systemctl reload smally.service
```

### View Logs

```bash
# View API logs
sudo journalctl -u smally.service -f

# View backup logs
sudo journalctl -u smally-backup.service

# View last 100 lines
sudo journalctl -u smally.service -n 100
```

### Manual Backup

```bash
# Trigger backup manually
sudo systemctl start smally-backup.service

# Check backup status
sudo systemctl status smally-backup.service
```

## Auto-start on Boot

Once enabled, Smally will automatically start when the server boots.

```bash
# Enable
sudo systemctl enable smally.service

# Disable
sudo systemctl disable smally.service

# Check if enabled
sudo systemctl is-enabled smally.service
```

## Troubleshooting

### Service Fails to Start

```bash
# Check detailed status
sudo systemctl status smally.service -l

# View recent logs
sudo journalctl -u smally.service -n 50

# Verify Docker is running
sudo systemctl status docker

# Check file permissions
ls -la /home/smally/smally-api/
```

### Backup Timer Not Running

```bash
# Check timer status
sudo systemctl status smally-backup.timer

# List all timers
systemctl list-timers --all

# Manually trigger backup
sudo systemctl start smally-backup.service
```

## Notes

- Service runs as `smally` user
- Backup runs daily at 2 AM
- Logs are stored in systemd journal
- Service automatically restarts on failure
