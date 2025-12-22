variable "project_id" {
  description = "GCP project ID"
  type        = string
}

variable "testnet_regions" {
  description = "Regions for testnet clusters"
  type        = list(string)
  default     = ["us-central1", "europe-west1"]
}

variable "mainnet_regions" {
  description = "Regions for mainnet clusters"
  type        = list(string)
  default     = ["us-central1", "europe-west1", "asia-east1"]
}
