terraform {
  required_version = ">= 1.0"

  required_providers {
    hcloud = {
      source  = "hetznercloud/hcloud"
      version = "~> 1.45"
    }
  }

  # Optional: Remote state backend
  # backend "s3" {
  #   bucket = "fastembed-terraform-state"
  #   key    = "production/terraform.tfstate"
  #   region = "eu-central-1"
  # }
}

provider "hcloud" {
  token = var.hcloud_token
}

# SSH Key for server access
resource "hcloud_ssh_key" "fastembed" {
  name       = "fastembed-${var.environment}"
  public_key = file(var.ssh_public_key_path)
}

# Firewall rules
resource "hcloud_firewall" "fastembed" {
  name = "fastembed-${var.environment}"

  # SSH
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = "22"
    source_ips = var.allowed_ssh_ips
  }

  # HTTP
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = "80"
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # HTTPS
  rule {
    direction = "in"
    protocol  = "tcp"
    port      = "443"
    source_ips = [
      "0.0.0.0/0",
      "::/0"
    ]
  }

  # Prometheus (optional, for monitoring from specific IPs)
  dynamic "rule" {
    for_each = var.enable_prometheus_access ? [1] : []
    content {
      direction = "in"
      protocol  = "tcp"
      port      = "9090"
      source_ips = var.monitoring_ips
    }
  }

  # Grafana (optional)
  dynamic "rule" {
    for_each = var.enable_grafana_access ? [1] : []
    content {
      direction = "in"
      protocol  = "tcp"
      port      = "3000"
      source_ips = var.monitoring_ips
    }
  }
}

# Main server
resource "hcloud_server" "fastembed" {
  name        = "fastembed-${var.environment}"
  server_type = var.server_type
  image       = var.server_image
  location    = var.location

  ssh_keys = [hcloud_ssh_key.fastembed.id]

  firewall_ids = [hcloud_firewall.fastembed.id]

  labels = {
    environment = var.environment
    application = "fastembed"
    managed_by  = "terraform"
  }

  # Cloud-init user data
  user_data = templatefile("${path.module}/cloud-init.yaml", {
    hostname    = "fastembed-${var.environment}"
    environment = var.environment
  })

  # Prevent accidental deletion in production
  lifecycle {
    prevent_destroy = false  # Set to true for production
  }
}

# Volume for persistent data (optional)
resource "hcloud_volume" "fastembed_data" {
  count    = var.enable_persistent_volume ? 1 : 0
  name     = "fastembed-data-${var.environment}"
  size     = var.volume_size
  location = var.location
  format   = "ext4"

  labels = {
    environment = var.environment
    application = "fastembed"
  }
}

# Attach volume to server
resource "hcloud_volume_attachment" "fastembed_data" {
  count     = var.enable_persistent_volume ? 1 : 0
  volume_id = hcloud_volume.fastembed_data[0].id
  server_id = hcloud_server.fastembed.id
  automount = true
}

# Floating IP (for easy migration)
resource "hcloud_floating_ip" "fastembed" {
  count         = var.enable_floating_ip ? 1 : 0
  type          = "ipv4"
  home_location = var.location

  labels = {
    environment = var.environment
    application = "fastembed"
  }
}

# Attach floating IP
resource "hcloud_floating_ip_assignment" "fastembed" {
  count          = var.enable_floating_ip ? 1 : 0
  floating_ip_id = hcloud_floating_ip.fastembed[0].id
  server_id      = hcloud_server.fastembed.id
}

# DNS A record (if using Cloudflare)
# Requires cloudflare provider
# resource "cloudflare_record" "fastembed" {
#   zone_id = var.cloudflare_zone_id
#   name    = var.domain_name
#   value   = hcloud_server.fastembed.ipv4_address
#   type    = "A"
#   ttl     = 300
#   proxied = var.cloudflare_proxy_enabled
# }
