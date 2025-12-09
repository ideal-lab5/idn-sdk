# Testnet clusters
resource "google_container_cluster" "testnet" {
  for_each = toset(var.testnet_regions)

  name     = "idn-testnet-${each.value}"
  location = each.value

  # Autopilot mode - Google manages node pools
  enable_autopilot = true

  # Network config
  network    = "default"
  subnetwork = "default"

  # Release channel for automatic upgrades
  release_channel {
    channel = "REGULAR"
  }

  # Vertical Pod Autoscaling
  vertical_pod_autoscaling {
    enabled = true
  }

  deletion_protection = true
}

# Mainnet clusters
resource "google_container_cluster" "mainnet" {
  for_each = toset(var.mainnet_regions)

  name     = "idn-mainnet-${each.value}"
  location = each.value

  # Autopilot mode - Google manages node pools
  enable_autopilot = true

  # Network config
  network    = "default"
  subnetwork = "default"

  # Release channel for automatic upgrades
  release_channel {
    channel = "STABLE"
  }

  # Vertical Pod Autoscaling
  vertical_pod_autoscaling {
    enabled = true
  }

  deletion_protection = true
}
