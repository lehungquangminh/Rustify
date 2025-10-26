# Rustify — URL Shortener hiệu năng cao (Rust + Axum)

[![CI](https://github.com/lehungquangminh/Rustify/actions/workflows/ci.yml/badge.svg)](https://github.com/lehungquangminh/Rustify/actions/workflows/ci.yml)
[![Release](https://github.com/lehungquangminh/Rustify/actions/workflows/release.yml/badge.svg)](https://github.com/lehungquangminh/Rustify/actions/workflows/release.yml)
[![CodeQL](https://github.com/lehungquangminh/Rustify/actions/workflows/codeql.yml/badge.svg)](https://github.com/lehungquangminh/Rustify/actions/workflows/codeql.yml)
[![Trivy](https://img.shields.io/badge/Trivy-Security%20Scan-4c1?logo=aqua&logoColor=white)](https://github.com/lehungquangminh/Rustify/actions/workflows/ci.yml)

Rustify là dịch vụ rút gọn liên kết hiệu năng cao, có thống kê (stats), rate limit, cache Redis, và quan sát hệ thống (Observability) đầy đủ, sẵn sàng cho Kubernetes.

- Ngôn ngữ/Framework: Rust + Axum
- DB: Postgres (sqlx + migrations)
- Cache: Redis (read-through)
- Observability: Prometheus metrics, OpenTelemetry tracing
- Bảo mật & Chất lượng: CodeQL, Trivy, Dependabot
- Đóng gói & Triển khai: Docker multi-stage (non-root), Helm chart, CI/CD GitHub Actions
- Hạ tầng mẫu: Terraform (AWS EKS, RDS Postgres, ElastiCache Redis, ECR, Route53, S3 tfstate + DynamoDB lock, OIDC role)

Tác giả: Lê Hùng Quang Minh

---

## Tính năng chính

- Rút gọn URL: `POST /shorten` trả về `alias` và `short_url`
- Điều hướng: `GET /:alias` redirect đến URL gốc
  - Trả QR PNG nếu `Accept: image/png`
- Thống kê: `GET /stats/:alias` tổng số click
- Rate limit: 60 req/phút/IP (mặc định 1 rps burst 60, có thể điều chỉnh)
- Cache Redis read-through cho alias -> URL
- Ghi nhận click theo lô (batch) mỗi giây để giảm tải ghi DB
- Prometheus `/metrics` và tracing OpenTelemetry

## Cấu trúc thư mục

```
.
├─ src/                  # Ứng dụng Axum
├─ migrations/           # Sqlx migrations
├─ docker/               # Dockerfile multi-stage (distroless, non-root)
├─ docker-compose.dev.yml
├─ deploy/
│  ├─ helm/              # Helm chart (Deployment, Service, Ingress, HPA, PDB, ServiceMonitor)
│  ├─ terraform/         # Terraform skeleton AWS (EKS, RDS, Redis, ECR, Route53, ALB, backend S3)
│  └─ external-secrets/  # ExternalSecret tham chiếu SSM
├─ .github/workflows/    # CI/CD, CodeQL
├─ Cargo.toml
├─ Makefile
└─ README.md
```

## Chạy local

Yêu cầu: Docker + Docker Compose, hoặc tự cung cấp Postgres/Redis.

- Docker Compose (khuyến nghị):

```bash
make docker-up
```

Mặc định:
- `DATABASE_URL=postgres://postgres:postgres@db:5432/rustify`
- `REDIS_URL=redis://cache:6379`
- `BASE_URL=http://localhost:8080`

- Chạy bằng cargo (tự có DB/Redis):

```bash
DATABASE_URL=postgres://... \
REDIS_URL=redis://... \
BASE_URL=http://localhost:8080 \
cargo run
```

## API

- Tạo short link

```http
POST /shorten
Content-Type: application/json

{
  "url": "https://example.com",
  "alias": "tùy_chọn"
}
```
Trả về:
```json
{ "alias": "abc123", "short_url": "http://localhost:8080/abc123" }
```

- Điều hướng hoặc QR
```http
GET /:alias
# Redirect 302 -> URL gốc
# Nếu gửi header: Accept: image/png => trả QR PNG
```

- Thống kê
```http
GET /stats/:alias
```
Trả về:
```json
{ "alias": "abc123", "url": "https://example.com", "clicks": 42 }
```

- Metrics
```http
GET /metrics
```

## Cài đặt/biến môi trường

- `DATABASE_URL` (bắt buộc)
- `REDIS_URL` (bắt buộc)
- `BASE_URL` (mặc định `http://localhost:8080`)
- `CACHE_TTL_SECS` (mặc định `600`)
- `PORT` (mặc định `8080`)
- `RUST_LOG` (ví dụ `info`)

## Helm chart (Kubernetes)

Chỉnh `deploy/helm/values.yaml` cho repo image, domain ingress, resources,…
Triển khai:

```bash
make helm-up
```

Yêu cầu cụm có Prometheus Operator (nếu bật `ServiceMonitor`), Ingress NGINX.

## CI/CD

- CI (`.github/workflows/ci.yml`): lint, test, audit, build, Trivy FS scan, push GHCR khi push `main`
- Release (`.github/workflows/release.yml`): build multi-arch, push GHCR, Helm deploy khi tag `v*.*.*`
  - Bước OIDC đến cluster cần cấu hình AWS IAM Role + kubeconfig

## Terraform (AWS skeleton)

Mặc định gồm: VPC, EKS (IRSA), RDS Postgres, ElastiCache Redis, ECR, ALB, Route53 record, backend S3 + DynamoDB lock.
Cần điền: `aws_region`, thông tin backend S3+DynamoDB, domain/hosted zone, sizing DB/Redis.

```bash
make tf-plan
```

## Bảo mật

- External Secrets Operator: `deploy/external-secrets/externalsecret.yaml` tham chiếu `ClusterSecretStore` tên `aws-ssm`
- SOPS + age: cập nhật `.sops.yaml` với khóa age của bạn
- CodeQL, Dependabot, Trivy trong CI

## Giấy phép

Phần mềm phát hành theo giấy phép MIT. Xem file LICENSE để biết chi tiết.
