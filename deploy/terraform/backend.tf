terraform {
  required_version = ">= 1.6.0"
  backend "s3" {
    bucket         = ""
    key            = "rustify/terraform.tfstate"
    region         = ""
    dynamodb_table = ""
    encrypt        = true
  }
}
