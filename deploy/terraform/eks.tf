module "eks" {
  source          = "terraform-aws-modules/eks/aws"
  version         = "~> 20.0"
  cluster_name    = var.cluster_name
  cluster_version = "1.30"
  vpc_id          = var.vpc_id
  subnet_ids      = var.private_subnet_ids
  eks_managed_node_groups = {
    default = {
      desired_size = 2
      min_size     = 2
      max_size     = 6
      instance_types = ["t3.large"]
    }
  }
  enable_irsa = true
}
