output "cluster_name" { value = var.cluster_name }
output "ecr_repository_url" { value = aws_ecr_repository.rustify.repository_url }
