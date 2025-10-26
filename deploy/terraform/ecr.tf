resource "aws_ecr_repository" "rustify" {
  name                 = "rustify"
  image_tag_mutability = "MUTABLE"
  image_scanning_configuration { scan_on_push = true }
}
