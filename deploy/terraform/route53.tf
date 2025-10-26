resource "aws_route53_record" "rustify" {
  zone_id = var.hosted_zone_id
  name    = var.domain
  type    = "A"
  alias {
    name                   = aws_lb.ingress_dns.dns_name
    zone_id                = aws_lb.ingress_dns.zone_id
    evaluate_target_health = true
  }
}
