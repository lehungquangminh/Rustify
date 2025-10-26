resource "aws_lb" "ingress_dns" {
  name               = "rustify-ingress"
  load_balancer_type = "application"
  subnets            = var.public_subnet_ids
}
