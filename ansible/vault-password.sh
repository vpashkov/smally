#!/bin/bash
# Ansible vault password script
# Reads vault password from ANSIBLE_VAULT_PASSWORD environment variable
# Usage: export ANSIBLE_VAULT_PASSWORD="your-vault-password"

if [ -z "$ANSIBLE_VAULT_PASSWORD" ]; then
    echo "Error: ANSIBLE_VAULT_PASSWORD environment variable not set" >&2
    exit 1
fi

echo "$ANSIBLE_VAULT_PASSWORD"
