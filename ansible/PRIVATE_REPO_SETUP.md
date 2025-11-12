# Private GitHub Repository Setup

To use a private GitHub repository with Ansible, you need to set up a deploy key.

## Step 1: Generate Deploy Key

On your local machine:

```bash
# Generate SSH key for GitHub deploy access
ssh-keygen -t ed25519 -C "deploy@fastembed-api" -f ~/.ssh/github_deploy_key

# Display the public key
cat ~/.ssh/github_deploy_key.pub
```

## Step 2: Add Deploy Key to GitHub

1. Go to your GitHub repository: `https://github.com/vpashkov/fastembed-api`
2. Navigate to **Settings** → **Deploy keys**
3. Click **Add deploy key**
4. Title: `Ansible Deploy Key`
5. Paste the **public key** from `~/.ssh/github_deploy_key.pub`
6. **DO NOT** check "Allow write access" (read-only is safer)
7. Click **Add key**

## Step 3: Add Private Key to Ansible Vault

Create or edit your vault:

```bash
cd ansible
ansible-vault edit group_vars/all/vault.yml
```

Add the private key content:

```yaml
# Existing secrets
vault_db_password: your_db_password
vault_secret_key: your_secret_key
vault_jwt_secret: your_jwt_secret
vault_admin_password: your_admin_password

# GitHub deploy key (entire private key content)
vault_github_deploy_key: |
  -----BEGIN OPENSSH PRIVATE KEY-----
  b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW
  ... (paste entire content of ~/.ssh/github_deploy_key)
  -----END OPENSSH PRIVATE KEY-----
```

**Important**: Paste the **entire private key** including the BEGIN/END lines.

To get the private key:

```bash
cat ~/.ssh/github_deploy_key
```

## Step 4: Update Inventory

The inventory has already been updated to use SSH URL:

```yaml
fastembed_git_repo: git@github.com:vpashkov/fastembed-api.git
fastembed_git_deploy_key: "{{ vault_github_deploy_key }}"
```

## Step 5: Deploy

Run Ansible as normal:

```bash
ansible-playbook -i inventory/hosts.yml playbook.yml --ask-vault-pass
```

The playbook will:
1. Create `.ssh` directory for `fastembed` user
2. Copy the deploy key to `/home/fastembed/.ssh/github_deploy_key`
3. Configure SSH to use the deploy key for `github.com`
4. Clone the private repository using SSH

## Verification

After deployment, SSH to the server and verify:

```bash
ssh root@your-server-ip

# Switch to fastembed user
su - fastembed

# Test GitHub access
ssh -T git@github.com
# Should output: "Hi vpashkov/fastembed-api! You've successfully authenticated..."

# Check repository
cd /home/fastembed/fastembed-api
git remote -v
# Should show: git@github.com:vpashkov/fastembed-api.git
```

## Security Notes

1. **Private key is encrypted** in Ansible Vault - never commit unencrypted
2. **Deploy key is read-only** - cannot push to repository (best practice)
3. **SSH config disables host key checking** - only for automated deployments
4. **Key is user-specific** - only `fastembed` user can access it
5. **Strict file permissions** - SSH key is mode 0600, .ssh is mode 0700

## Troubleshooting

**Error: "Permission denied (publickey)"**

```bash
# SSH to server
ssh root@server-ip

# Check deploy key exists
ls -la /home/fastembed/.ssh/
# Should see: github_deploy_key (mode 600)

# Check SSH config
cat /home/fastembed/.ssh/config
# Should have GitHub configuration

# Test as fastembed user
su - fastembed
ssh -T git@github.com
```

**Error: "Repository not found"**

- Verify URL is SSH format: `git@github.com:user/repo.git`
- Check deploy key is added to correct repository on GitHub
- Ensure deploy key hasn't expired or been revoked

**Error: "Host key verification failed"**

```bash
# SSH to server and accept GitHub's host key
su - fastembed
ssh-keyscan github.com >> ~/.ssh/known_hosts
```

## Alternative: Personal Access Token (Not Recommended)

If you can't use deploy keys, use a Personal Access Token:

```yaml
# In vault.yml
vault_github_token: ghp_xxxxxxxxxxxxxxxxxxxx

# In inventory
fastembed_git_repo: https://{{ vault_github_token }}@github.com/vpashkov/fastembed-api.git
```

**Warning**: This is less secure as the token has broader permissions.

## Cleanup

To remove the deploy key from your local machine after adding to vault:

```bash
# ONLY do this after confirming Ansible deployment works!
rm ~/.ssh/github_deploy_key
rm ~/.ssh/github_deploy_key.pub
```

Keep a backup of the private key in a secure password manager in case you need to update the vault.
