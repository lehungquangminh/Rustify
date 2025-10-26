module "redis" {
  source  = "terraform-aws-modules/elasticache/aws"
  version = "~> 6.3"

  engine                 = "redis"
  engine_version         = "7.1"
  node_type              = var.redis_node_type
  num_cache_nodes        = 1
  subnet_ids             = var.private_subnet_ids
  security_group_ids     = []
}
