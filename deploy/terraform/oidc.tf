data "aws_iam_openid_connect_provider" "eks" {
  arn = module.eks.oidc_provider_arn
}
resource "aws_iam_role" "github_actions" {
  name               = "rustify-gha-oidc"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = { Federated = data.aws_iam_openid_connect_provider.eks.arn }
      Action = "sts:AssumeRoleWithWebIdentity"
      Condition = {
        StringEquals = { "token.actions.githubusercontent.com:aud" = "sts.amazonaws.com" }
      }
    }]
  })
}
