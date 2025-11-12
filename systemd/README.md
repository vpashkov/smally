# Systemd Service Files

These systemd service files allow you to manage FastEmbed API as a system service.

## Installation

### 1. Copy Service Files

```bash
sudo cp systemd/fastembed.service /etc/systemd/system/
sudo cp systemd/fastembed-backup.service /etc/systemd/system/
sudo cp systemd/fastembed-backup.timer /etc/systemd/system/
```

### 2. Reload Systemd

```bash
sudo systemctl daemon-reload
```

### 3. Enable Services

```bash
# Enable FastEmbed API to start on boot
sudo systemctl enable fastembed.service

# Enable automatic daily backups
sudo systemctl enable fastembed-backup.timer
```

### 4. Start Services

```bash
# Start FastEmbed API
sudo systemctl start fastembed.service

# Start backup timer
sudo systemctl start fastembed-backup.timer
```

## Usage

### Check Status

```bash
# Check API service status
sudo systemctl status fastembed.service

# Check backup timer status
sudo systemctl status fastembed-backup.timer

# List all timers
sudo systemctl list-timers
```

### Start/Stop/Restart

```bash
# Start
sudo systemctl start fastembed.service

# Stop
sudo systemctl stop fastembed.service

# Restart
sudo systemctl restart fastembed.service

# Reload configuration
sudo systemctl reload fastembed.service
```

### View Logs

```bash
# View API logs
sudo journalctl -u fastembed.service -f

# View backup logs
sudo journalctl -u fastembed-backup.service

# View last 100 lines
sudo journalctl -u fastembed.service -n 100
```

### Manual Backup

```bash
# Trigger backup manually
sudo systemctl start fastembed-backup.service

# Check backup status
sudo systemctl status fastembed-backup.service
```

## Auto-start on Boot

Once enabled, FastEmbed will automatically start when the server boots.

```bash
# Enable
sudo systemctl enable fastembed.service

# Disable
sudo systemctl disable fastembed.service

# Check if enabled
sudo systemctl is-enabled fastembed.service
```

## Troubleshooting

### Service Fails to Start

```bash
# Check detailed status
sudo systemctl status fastembed.service -l

# View recent logs
sudo journalctl -u fastembed.service -n 50

# Verify Docker is running
sudo systemctl status docker

# Check file permissions
ls -la /home/fastembed/fastembed-api/
```

### Backup Timer Not Running

```bash
# Check timer status
sudo systemctl status fastembed-backup.timer

# List all timers
systemctl list-timers --all

# Manually trigger backup
sudo systemctl start fastembed-backup.service
```

## Notes

- Service runs as `fastembed` user
- Backup runs daily at 2 AM
- Logs are stored in systemd journal
- Service automatically restarts on failure
