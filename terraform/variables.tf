variable "hcloud_token" {
  description = "Hetzner Cloud API Token"
  type        = string
  sensitive   = true
}

variable "environment" {
  description = "Environment name (production, staging, dev)"
  type        = string
  default     = "production"
}

variable "server_type" {
  description = "Hetzner server type"
  type        = string
  default     = "ccx33"  # 8 vCPU, 32GB RAM

  validation {
    condition     = can(regex("^(cx|cpx|ccx|cax)[0-9]+$", var.server_type))
    error_message = "Server type must be a valid Hetzner server type."
  }
}

variable "server_image" {
  description = "Server OS image"
  type        = string
  default     = "ubuntu-24.04"
}

variable "location" {
  description = "Hetzner datacenter location"
  type        = string
  default     = "nbg1"  # Nuremberg

  validation {
    condition     = contains(["nbg1", "fsn1", "hel1", "ash", "hil"], var.location)
    error_message = "Location must be a valid Hetzner location."
  }
}

variable "ssh_public_key_path" {
  description = "Path to SSH public key"
  type        = string
  default     = "~/.ssh/id_ed25519.pub"
}

variable "allowed_ssh_ips" {
  description = "IP addresses allowed to SSH to the server"
  type        = list(string)
  default     = ["0.0.0.0/0", "::/0"]  # Allow all - restrict in production!
}

variable "monitoring_ips" {
  description = "IP addresses allowed to access monitoring endpoints"
  type        = list(string)
  default     = []
}

variable "enable_prometheus_access" {
  description = "Enable external access to Prometheus (port 9090)"
  type        = bool
  default     = false
}

variable "enable_grafana_access" {
  description = "Enable external access to Grafana (port 3000)"
  type        = bool
  default     = false
}

variable "enable_persistent_volume" {
  description = "Create and attach persistent volume for data"
  type        = bool
  default     = true
}

variable "volume_size" {
  description = "Size of persistent volume in GB"
  type        = number
  default     = 100

  validation {
    condition     = var.volume_size >= 10 && var.volume_size <= 10000
    error_message = "Volume size must be between 10 and 10000 GB."
  }
}

variable "enable_floating_ip" {
  description = "Create floating IP for easy server migration"
  type        = bool
  default     = true
}

variable "domain_name" {
  description = "Domain name for the API (e.g., api.smally.io)"
  type        = string
  default     = ""
}

variable "cloudflare_zone_id" {
  description = "Cloudflare zone ID for DNS management"
  type        = string
  default     = ""
  sensitive   = true
}

variable "cloudflare_proxy_enabled" {
  description = "Enable Cloudflare proxy (orange cloud)"
  type        = bool
  default     = false
}
