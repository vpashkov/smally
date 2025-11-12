output "server_id" {
  description = "Hetzner server ID"
  value       = hcloud_server.fastembed.id
}

output "server_name" {
  description = "Server name"
  value       = hcloud_server.fastembed.name
}

output "server_ipv4" {
  description = "Server IPv4 address"
  value       = hcloud_server.fastembed.ipv4_address
}

output "server_ipv6" {
  description = "Server IPv6 address"
  value       = hcloud_server.fastembed.ipv6_address
}

output "floating_ip" {
  description = "Floating IP address (if enabled)"
  value       = var.enable_floating_ip ? hcloud_floating_ip.fastembed[0].ip_address : null
}

output "ssh_command" {
  description = "SSH command to connect to server"
  value       = "ssh root@${hcloud_server.fastembed.ipv4_address}"
}

output "api_url" {
  description = "API URL (using domain or IP)"
  value       = var.domain_name != "" ? "https://${var.domain_name}/v1/embed" : "https://${hcloud_server.fastembed.ipv4_address}/v1/embed"
}

output "grafana_url" {
  description = "Grafana dashboard URL"
  value       = "http://${hcloud_server.fastembed.ipv4_address}:3000"
}

output "prometheus_url" {
  description = "Prometheus metrics URL"
  value       = "http://${hcloud_server.fastembed.ipv4_address}:9090"
}

output "ansible_inventory" {
  description = "Ansible inventory entry"
  value       = <<-EOT
    [fastembed]
    ${hcloud_server.fastembed.name} ansible_host=${hcloud_server.fastembed.ipv4_address} ansible_user=root
  EOT
}
