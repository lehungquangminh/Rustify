module "db" {
  source  = "terraform-aws-modules/rds/aws"
  version = "~> 6.5"

  identifier = "rustify"
  engine               = "postgres"
  engine_version       = "16"
  family               = "postgres16"
  instance_class       = var.db_instance_class
  allocated_storage    = 20
  db_name              = var.db_name
  username             = var.db_username
  manage_master_user_password = true
  create_db_subnet_group      = true
  subnet_ids                  = var.private_subnet_ids
  vpc_security_group_ids      = []
  publicly_accessible         = false
  deletion_protection         = false
  skip_final_snapshot         = true
}
