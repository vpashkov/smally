# Terraform Configuration for Smally API

Infrastructure provisioning for Hetzner Cloud using Terraform.

## Quick Start

```bash
# 1. Copy example variables
cp terraform.tfvars.example terraform.tfvars

# 2. Edit with your values
vim terraform.tfvars

# 3. Initialize Terraform
terraform init

# 4. Preview changes
terraform plan

# 5. Create infrastructure
terraform apply
```

## What It Creates

- **Server**: Hetzner Cloud CCX server (configurable type)
- **SSH Key**: Uploaded to Hetzner for access
- **Firewall**: Rules for SSH, HTTP, HTTPS, monitoring ports
- **Floating IP**: Optional static IP
- **Volume**: Optional additional storage
- **Cloud-init**: Automated initial server setup

## Prerequisites

1. **Hetzner Cloud Account**
   - Sign up: <https://console.hetzner.cloud>
   - Create a project
   - Generate API token: Project → Security → API Tokens

2. **SSH Key**

   ```bash
   ssh-keygen -t ed25519 -C "your_email@example.com"
   cat ~/.ssh/id_ed25519.pub  # Copy this for terraform.tfvars
   ```

3. **Terraform**

   ```bash
   # macOS
   brew install terraform
   # or OpenTofu
   brew install opentofu
   ```

## Configuration

### Required Variables

Edit `terraform.tfvars`:

```hcl
# Hetzner API token (from console.hetzner.cloud)
hcloud_token = "your_hetzner_api_token_here"

# Your SSH public key
ssh_public_key = "ssh-ed25519 AAAA... your_email@example.com"

# Optional: restrict SSH access to specific IPs
allowed_ssh_ips = [
  "1.2.3.4/32",      # Your office IP
  "5.6.7.8/32"       # Your home IP
]
```

### Optional Variables

```hcl
# Server configuration
server_type = "ccx33"          # Default: 8 vCPU, 32GB RAM, 240GB SSD
server_image = "ubuntu-22.04"  # Default: Ubuntu 22.04
location = "nbg1"              # Default: Nuremberg, Germany
environment = "production"     # Default: production

# Additional storage
create_volume = false          # Set to true to create 100GB volume
volume_size = 100             # Size in GB
```

### Server Types

| Type   | vCPU | RAM   | Disk   | Network  | Price/mo |
|--------|------|-------|--------|----------|----------|
| cpx21  | 3    | 4GB   | 80GB   | 20TB     | €7.50    |
| cpx31  | 4    | 8GB   | 160GB  | 20TB     | €14.40   |
| ccx23  | 4    | 16GB  | 80GB   | 20TB     | €24.90   |
| ccx33  | 8    | 32GB  | 240GB  | 20TB     | €48.90   |
| ccx43  | 16   | 64GB  | 360GB  | 20TB     | €97.90   |

**Recommended**: `ccx33` for production (500+ QPS, 10M req/day)

### Locations

- `nbg1`: Nuremberg, Germany
- `fsn1`: Falkenstein, Germany
- `hel1`: Helsinki, Finland
- `ash`: Ashburn, VA, USA
- `hil`: Hillsboro, OR, USA

## Usage

### Initialize

```bash
terraform init
```

Downloads providers and modules.

### Plan

```bash
terraform plan
```

Preview changes before applying. Shows:

- Resources to be created
- Estimated costs
- Changes to existing resources

### Apply

```bash
terraform apply
```

Creates infrastructure. Confirm with `yes`.

### Outputs

After apply, important values are displayed:

```
Outputs:

server_ipv4 = "X.X.X.X"
server_ipv6 = "Y:Y:Y::1"
server_name = "smally-production"
ssh_command = "ssh root@X.X.X.X"
firewall_id = "123456"
```

### Show Outputs Later

```bash
terraform output

# Specific output
terraform output server_ipv4

# For scripts
export SERVER_IP=$(terraform output -raw server_ipv4)
```

### Destroy

```bash
terraform destroy
```

**Warning**: Deletes ALL resources. Backup data first!

## Cloud-init

The server is bootstrapped with `cloud-init.yaml`:

1. Updates packages
2. Installs Python, Docker dependencies
3. Configures hostname
4. Sets up firewall (UFW)
5. Configures kernel parameters
6. Creates swap file (2GB)
7. Enables unattended security updates

Cloud-init takes 2-3 minutes. Check status:

```bash
ssh root@server-ip "cloud-init status"
# Output: status: done
```

## Firewall Rules

Created automatically:

| Direction | Protocol | Port  | Source      | Description      |
|-----------|----------|-------|-------------|------------------|
| Inbound   | TCP      | 22    | allowed_ips | SSH              |
| Inbound   | TCP      | 80    | 0.0.0.0/0   | HTTP             |
| Inbound   | TCP      | 443   | 0.0.0.0/0   | HTTPS            |
| Inbound   | TCP      | 9090  | allowed_ips | Prometheus       |
| Inbound   | TCP      | 3000  | allowed_ips | Grafana          |
| Inbound   | ICMP     | -     | 0.0.0.0/0   | Ping             |

**Security**: SSH, Prometheus, and Grafana are restricted to `allowed_ssh_ips`.

## State Management

### Local State (Default)

State stored in `terraform.tfstate` (gitignored).

**Backup regularly**:

```bash
cp terraform.tfstate terraform.tfstate.backup
```

### Remote State (Recommended for Teams)

Use S3 backend:

Create `backend.tf`:

```hcl
terraform {
  backend "s3" {
    bucket         = "my-terraform-state"
    key            = "smally/production/terraform.tfstate"
    region         = "eu-central-1"
    encrypt        = true
    dynamodb_table = "terraform-locks"
  }
}
```

Initialize:

```bash
terraform init -migrate-state
```

## Advanced Usage

### Multiple Environments

Create separate configs:

```
terraform/
├── environments/
│   ├── production/
│   │   └── terraform.tfvars
│   └── staging/
│       └── terraform.tfvars
```

Apply per environment:

```bash
terraform apply -var-file=environments/production/terraform.tfvars
```

### Import Existing Resources

If you created resources manually:

```bash
# Import server
terraform import hcloud_server.smally 12345

# Import firewall
terraform import hcloud_firewall.smally 67890
```

### Workspace Isolation

```bash
# Create staging workspace
terraform workspace new staging
terraform apply

# Switch to production
terraform workspace select production
terraform apply

# List workspaces
terraform workspace list
```

## Troubleshooting

### Authentication Error

**Error**: `Error: unable to fetch token: invalid token`

**Fix**: Check `hcloud_token` in `terraform.tfvars`:

```bash
# Test token with Hetzner CLI
export HCLOUD_TOKEN="your_token"
hcloud server list
```

### Server Already Exists

**Error**: `Error: server name already exists`

**Fix**:

1. Import: `terraform import hcloud_server.smally <id>`
2. Or destroy: `terraform destroy`
3. Or rename in variables: `server_name = "smally-v2"`

### SSH Key Exists

**Error**: `Error: SSH key already exists`

**Fix**:

```bash
# List keys
hcloud ssh-key list

# Import
terraform import hcloud_ssh_key.smally <key-id>

# Or delete old key
hcloud ssh-key delete <key-id>
```

### Rate Limit

**Error**: `Error: too many requests`

**Fix**: Wait 60 seconds, then retry. Hetzner limits: 3600 requests/hour.

### Region Unavailable

**Error**: `Error: server type not available in location`

**Fix**: Change `location` in `terraform.tfvars`:

```hcl
location = "fsn1"  # Try different location
```

## Cost Estimation

Use Terraform Cloud or Infracost:

```bash
# Install Infracost
brew install infracost

# Estimate costs
infracost breakdown --path .
```

Expected costs (ccx33):

- Server: €48.90/month
- Floating IP (optional): €1.19/month
- Volume 100GB (optional): €4.80/month
- Backups (optional): €9.78/month (20% of server cost)
- **Total**: ~€50-65/month

## Best Practices

1. **Use remote state** for team collaboration
2. **Version control** terraform.tfvars (encrypted) or use env vars
3. **Tag resources** with environment, team, cost center
4. **Enable backups** for production servers
5. **Use workspaces** or separate state files per environment
6. **Document changes** in commit messages
7. **Review plans** before applying
8. **Regular backups** of terraform.tfstate

## Integration with Ansible

After Terraform creates infrastructure:

```bash
# Get server IP
export SERVER_IP=$(terraform output -raw server_ipv4)

# Update Ansible inventory
cd ../ansible
sed -i "s/YOUR_SERVER_IP/$SERVER_IP/" inventory/hosts.yml

# Run Ansible
ansible-playbook -i inventory/hosts.yml playbook.yml --ask-vault-pass
```

## Next Steps

1. **Apply Terraform**: `terraform apply`
2. **Note outputs**: Save server IP and SSH command
3. **Wait for cloud-init**: 2-3 minutes
4. **Run Ansible**: Configure server with `../ansible/playbook.yml`
5. **Verify deployment**: Access API at `http://server-ip:8000/health`

## Documentation

- Full guide: [../docs/TERRAFORM_ANSIBLE.md](../docs/TERRAFORM_ANSIBLE.md)
- Deployment guide: [../docs/deployment.md](../docs/deployment.md)
- Infrastructure overview: [../docs/INFRASTRUCTURE.md](../docs/INFRASTRUCTURE.md)

## Resources

- [Terraform Hetzner Provider](https://registry.terraform.io/providers/hetznercloud/hcloud/latest/docs)
- [Hetzner Cloud Console](https://console.hetzner.cloud/)
- [Hetzner Cloud Pricing](https://www.hetzner.com/cloud)
- [Terraform Documentation](https://www.terraform.io/docs)
