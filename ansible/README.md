# Ansible Deployment for FastEmbed API

Automated server configuration and application deployment using Ansible.

## Quick Start

```bash
# 1. Install dependencies
pip install ansible
ansible-galaxy collection install community.docker

# 2. Update inventory with your server IP
vim inventory/hosts.yml

# 3. Set vault password and create encrypted secrets
export ANSIBLE_VAULT_PASSWORD="your-strong-vault-password"
cp group_vars/all/vars.yml.example group_vars/all/vault.yml
ansible-vault edit group_vars/all/vault.yml

# 4. Run playbook
ansible-playbook -i inventory/hosts.yml playbook.yml
```

## What It Does

This Ansible playbook configures a production-ready FastEmbed API server:

1. **Common Role**: OS hardening, firewall, fail2ban, unattended upgrades
2. **Docker Role**: Docker and Docker Compose installation
3. **FastEmbed Role**: Application deployment, SSL setup, systemd services

## Requirements

- Ansible 2.14+
- Python 3.8+
- Target server: Ubuntu 22.04 (fresh installation)
- SSH access as root

## Inventory Configuration

Edit `inventory/hosts.yml`:

```yaml
fastembed-prod:
  ansible_host: X.X.X.X  # Your server IP
  fastembed_domain: api.yourdomain.com
  fastembed_git_repo: https://github.com/user/fastembed-api.git
```

## Secrets Management

All sensitive data is stored in an encrypted Ansible Vault file. Set your vault password as an environment variable:

```bash
export ANSIBLE_VAULT_PASSWORD="your-vault-password"
```

Then run the playbook normally (no `--ask-vault-pass` needed):

```bash
ansible-playbook -i inventory/hosts.yml playbook.yml
```

For detailed instructions on creating and managing secrets, see **[SECRET_MANAGEMENT.md](SECRET_MANAGEMENT.md)**

Quick setup:
```bash
# Copy the template
cp group_vars/all/vars.yml.example group_vars/all/vault.yml

# Edit and encrypt
export ANSIBLE_VAULT_PASSWORD="your-strong-password"
ansible-vault edit group_vars/all/vault.yml
```

## SSL Certificates

### Option 1: Self-Signed (Development)

```yaml
fastembed_ssl_selfsigned: true
```

### Option 2: Let's Encrypt (Production)

```yaml
fastembed_ssl_selfsigned: false
fastembed_ssl_cert: /etc/letsencrypt/live/yourdomain.com/fullchain.pem
fastembed_ssl_key: /etc/letsencrypt/live/yourdomain.com/privkey.pem
```

## Playbook Tags

Run specific parts:

```bash
# Only common setup
ansible-playbook playbook.yml --tags common

# Only Docker
ansible-playbook playbook.yml --tags docker

# Only FastEmbed
ansible-playbook playbook.yml --tags fastembed
```

## Roles

### Common

- Creates `fastembed` user
- Configures UFW firewall (ports 22, 80, 443, 9090, 3000)
- Sets kernel parameters for performance
- Installs fail2ban for SSH protection
- Enables unattended security updates

### Docker

- Adds Docker APT repository
- Installs Docker CE and Docker Compose
- Configures Docker daemon (logging, storage driver)
- Creates Docker network for FastEmbed
- Adds fastembed user to docker group

### FastEmbed

- Clones Git repository
- Creates `.env.production` from template
- Generates or copies SSL certificates
- Runs deployment script
- Installs systemd services
- Enables automated backups

## Post-Deployment

Verify deployment:

```bash
# SSH to server
ssh root@your-server-ip

# Check services
systemctl status fastembed
docker ps

# Test API
curl http://localhost:8000/health

# View logs
docker-compose -f /home/fastembed/fastembed-api/docker-compose.prod.yml logs -f
```

## Updating

Re-run the playbook to update:

```bash
ansible-playbook -i inventory/hosts.yml playbook.yml --ask-vault-pass
```

This will:

- Pull latest code from Git
- Rebuild and restart containers
- Apply any configuration changes

## Variables

See `roles/*/defaults/main.yml` for all available variables.

Key variables:

- `fastembed_workers`: Number of Uvicorn workers (default: 4)
- `fastembed_model_name`: Embedding model (default: BAAI/bge-small-en-v1.5)
- `fastembed_rate_limit_*`: Rate limits per tier
- `fastembed_app_dir`: Application directory (default: /home/fastembed/fastembed-api)

## Troubleshooting

**Connection failed**

```bash
# Test SSH
ssh root@server-ip

# Test ping
ansible fastembed -i inventory/hosts.yml -m ping
```

**Vault password error**

```bash
# Edit vault
ansible-vault edit group_vars/all/vault.yml

# Change password
ansible-vault rekey group_vars/all/vault.yml
```

**Docker permission denied**

```bash
# Re-run playbook to fix permissions
ansible-playbook playbook.yml --tags docker
```

## Documentation

Full documentation: [../docs/TERRAFORM_ANSIBLE.md](../docs/TERRAFORM_ANSIBLE.md)
