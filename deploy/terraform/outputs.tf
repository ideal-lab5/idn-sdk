output "testnet_clusters" {
  description = "Testnet cluster details"
  value = {
    for region, cluster in google_container_cluster.testnet : region => {
      name     = cluster.name
      endpoint = cluster.endpoint
      location = cluster.location
    }
  }
}

output "mainnet_clusters" {
  description = "Mainnet cluster details"
  value = {
    for region, cluster in google_container_cluster.mainnet : region => {
      name     = cluster.name
      endpoint = cluster.endpoint
      location = cluster.location
    }
  }
}

output "testnet_cluster_names" {
  description = "List of testnet cluster names for CI/CD"
  value       = [for cluster in google_container_cluster.testnet : cluster.name]
}

output "mainnet_cluster_names" {
  description = "List of mainnet cluster names for CI/CD"
  value       = [for cluster in google_container_cluster.mainnet : cluster.name]
}
