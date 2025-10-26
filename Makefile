SHELL := /bin/sh
APP := rustify
REGISTRY ?= ghcr.io/$(USER)
IMAGE ?= $(REGISTRY)/$(APP):latest

build:
	cargo build

run:
	cargo run

docker-build:
	docker build -t $(IMAGE) -f docker/Dockerfile .

docker-up:
	docker compose -f docker-compose.dev.yml up --build

helm-up:
	helm upgrade --install rustify deploy/helm -n rustify --create-namespace

tf-plan:
	cd deploy/terraform && terraform init && terraform plan
