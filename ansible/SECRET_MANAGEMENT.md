# Secret Management Guide

This guide explains how to securely manage secrets for the Smally API deployment using Ansible Vault.

## Overview

All sensitive data (passwords, API keys, SSH keys) are stored in an encrypted Ansible Vault file. The vault password is provided via an environment variable, enabling both manual deployments and CI/CD automation.

## Files

- **`group_vars/all/vault.yml`** - Encrypted secrets file (tracked in git)
- **`group_vars/all/vars.yml.example`** - Template showing required variables (tracked in git)
- **`vault-password.sh`** - Script to read vault password from environment variable
- **`ansible.cfg`** - Configured to use vault-password.sh

## Setup for New Deployments

### 1. Set Vault Password

```bash
# Create a strong vault password (save this securely!)
export ANSIBLE_VAULT_PASSWORD="your-strong-vault-password"

# For persistent sessions, add to ~/.bashrc or ~/.zshrc:
echo 'export ANSIBLE_VAULT_PASSWORD="your-strong-vault-password"' >> ~/.bashrc
```

### 2. View Current Vault (Optional)

If the vault already exists and you have the password:

```bash
cd ansible
ansible-vault view group_vars/all/vault.yml
```

### 3. Create New Vault

If starting fresh:

```bash
cd ansible

# Copy the example template
cp group_vars/all/vars.yml.example group_vars/all/vault.yml

# Edit the vault file to add your secrets
ansible-vault edit group_vars/all/vault.yml
```

This will:
1. Encrypt the file using your ANSIBLE_VAULT_PASSWORD
2. Open it in your default editor (usually vim)
3. Save and encrypt when you exit

### 4. Required Secrets

Your `vault.yml` must contain these variables:

```yaml
# Database
vault_db_password: strong_database_password

# Application secrets (generate with: openssl rand -base64 32)
vault_secret_key: base64_encoded_secret_key
vault_jwt_secret: base64_encoded_jwt_secret

# Admin credentials
vault_admin_password: admin_user_password

# GitHub deploy key (for private repositories)
vault_github_deploy_key: |
  -----BEGIN OPENSSH PRIVATE KEY-----
  your_private_deploy_key_here
  -----END OPENSSH PRIVATE KEY-----
```

### 5. Generate Secure Values

```bash
# Database password
openssl rand -base64 32

# Secret key
openssl rand -base64 32

# JWT secret
openssl rand -base64 32

# GitHub deploy key (for private repos)
ssh-keygen -t ed25519 -C "deploy@smally-api" -f ~/.ssh/github_deploy_key
# Add the public key (~/.ssh/github_deploy_key.pub) to GitHub: Settings > Deploy keys
# Use the private key (~/.ssh/github_deploy_key) in vault.yml
```

## Using the Vault

### Edit Secrets

```bash
cd ansible
export ANSIBLE_VAULT_PASSWORD="your-vault-password"
ansible-vault edit group_vars/all/vault.yml
```

### View Secrets

```bash
cd ansible
export ANSIBLE_VAULT_PASSWORD="your-vault-password"
ansible-vault view group_vars/all/vault.yml
```

### Run Playbook

The playbook will automatically decrypt the vault using the password from the environment variable:

```bash
export ANSIBLE_VAULT_PASSWORD="your-vault-password"
ansible-playbook -i inventory/hosts.yml playbook.yml
```

### Rotate Vault Password

To change the vault encryption password:

```bash
cd ansible
export ANSIBLE_VAULT_PASSWORD="old-password"
export NEW_ANSIBLE_VAULT_PASSWORD="new-password"

# Rekey the vault
ansible-vault rekey group_vars/all/vault.yml --new-vault-password-file <(echo "$NEW_ANSIBLE_VAULT_PASSWORD")

# Update your environment
export ANSIBLE_VAULT_PASSWORD="new-password"
```

## CI/CD Integration

### GitHub Actions

Add the vault password as a GitHub Secret:

1. Go to your repository Settings > Secrets and variables > Actions
2. Add a new secret: `ANSIBLE_VAULT_PASSWORD`
3. Use it in your workflow:

```yaml
- name: Deploy with Ansible
  env:
    ANSIBLE_VAULT_PASSWORD: ${{ secrets.ANSIBLE_VAULT_PASSWORD }}
  run: |
    cd ansible
    ansible-playbook -i inventory/hosts.yml playbook.yml
```

### GitLab CI

Add the vault password as a masked CI/CD variable:

1. Go to Settings > CI/CD > Variables
2. Add variable: `ANSIBLE_VAULT_PASSWORD` (mark as Masked and Protected)
3. Use in `.gitlab-ci.yml`:

```yaml
deploy:
  script:
    - cd ansible
    - ansible-playbook -i inventory/hosts.yml playbook.yml
  variables:
    ANSIBLE_VAULT_PASSWORD: $ANSIBLE_VAULT_PASSWORD
```

## Security Best Practices

1. **Never commit unencrypted secrets** - All secrets must be in vault.yml
2. **Use strong vault password** - At least 32 characters, random
3. **Rotate secrets regularly** - Database passwords, API keys, etc.
4. **Limit vault access** - Only grant vault password to authorized team members
5. **Use different secrets per environment** - Separate vaults for dev/staging/prod
6. **Backup vault password** - Store securely (password manager, secrets manager)
7. **Audit vault changes** - Review git history for vault.yml modifications

## Troubleshooting

### Error: "ANSIBLE_VAULT_PASSWORD environment variable not set"

```bash
# Set the environment variable
export ANSIBLE_VAULT_PASSWORD="your-vault-password"
```

### Error: "Decryption failed"

- Verify you're using the correct vault password
- Check if the vault file is corrupted: `ansible-vault view group_vars/all/vault.yml`

### Error: "Vault format unhexlify error"

The vault file is corrupted or not properly encrypted. Restore from backup or recreate:

```bash
# If you know the values
ansible-vault create group_vars/all/vault.yml
# Enter your secrets manually
```

### Forgot Vault Password

If you lose the vault password:
1. You cannot decrypt the existing vault
2. You must create a new vault with new secrets
3. Update all affected systems with the new secrets

## Alternative: Local Unencrypted Vault (Development Only)

For local development, you can use an unencrypted vars.yml file:

```bash
cd ansible
cp group_vars/all/vars.yml.example group_vars/all/vars.yml
# Edit vars.yml with your secrets (NOT RECOMMENDED for production)
```

**WARNING:** `vars.yml` is gitignored, but never commit unencrypted secrets to version control!

## Migrating from vars.yml to vault.yml

If you have an existing unencrypted `vars.yml`:

```bash
cd ansible

# Set your vault password
export ANSIBLE_VAULT_PASSWORD="your-new-vault-password"

# Encrypt the existing file
ansible-vault encrypt group_vars/all/vars.yml

# Rename to vault.yml
mv group_vars/all/vars.yml group_vars/all/vault.yml

# Commit the encrypted vault
git add group_vars/all/vault.yml
git commit -m "Encrypt secrets with Ansible Vault"
```

## FAQ

**Q: Can I have multiple vault files?**
A: Yes, Ansible loads all `.yml` files in `group_vars/all/`. You can have:
- `vault.yml` - Encrypted secrets
- `vars.yml` - Unencrypted non-sensitive variables

**Q: Should I commit vault.yml to git?**
A: Yes, the encrypted `vault.yml` is safe to commit. Never commit unencrypted secrets.

**Q: How do I share the vault password with my team?**
A: Use a password manager (1Password, LastPass) or secrets manager (HashiCorp Vault, AWS Secrets Manager).

**Q: Can I use a file instead of environment variable?**
A: Yes, update `ansible.cfg`:
```ini
vault_password_file = /path/to/password/file
```
But environment variables are more secure for CI/CD.

**Q: What if someone gets access to my vault.yml?**
A: The file is encrypted with AES256. Without the vault password, they cannot decrypt it. However, rotate your secrets as a precaution.
