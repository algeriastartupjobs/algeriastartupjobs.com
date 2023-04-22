data "terraform_remote_state" "shared" {
  backend = "local"
  config  = { path = "${path.module}/../shared/terraform.tfstate.d/shared/terraform.tfstate" }
}

terraform {
  required_providers {
    digitalocean = {
      source  = "digitalocean/digitalocean"
      version = "~> 2.0"
    }
    ssh = {
      source = "loafoe/ssh"
    }
  }
}

variable "do_api_key" {
  description = "The API key for the DigitalOcean account"
  type        = string
  sensitive   = true
}

variable "do_ssh_pub_key" {
  description = "The SSH key for the DigitalOcean droptlet"
  type        = string
  sensitive   = true
}

variable "do_ssh_key" {
  description = "The private SSH key for the DigitalOcean droptlet"
  type        = string
  sensitive   = true
}

variable "do_droplet_user" {
  description = "The user for the DigitalOcean droplet"
  type        = string
  sensitive   = true
}

variable "do_droplet_password" {
  description = "The password for the DigitalOcean droplet's user"
  type        = string
  sensitive   = true
}

locals {
  app_folder_name  = "asj"
  app_folder       = "~/${local.app_folder_name}"
  upload_tmp_name  = "~/upload_tmp"
  app_name         = "algeriastartupjobs-api"
  service_name     = "algeriastartupjobs-api"
  stage            = terraform.workspace
  root_domain_name = "algeriastartupjobs.com"
  sub_domain_name  = local.stage == "production" ? "api" : "api.${local.stage}"
  domain_name      = "${local.sub_domain_name}.${local.root_domain_name}"
}

provider "digitalocean" {
  token = var.do_api_key
}

provider "aws" {
  region = "eu-west-1"
}

resource "digitalocean_droplet" "api" {
  image     = "debian-11-x64"
  name      = local.domain_name
  region    = "fra1"
  size      = "s-1vcpu-512mb-10gb"
  ssh_keys  = [data.terraform_remote_state.shared.outputs.digitalocean_ssh_key_fingerprint]
  user_data = <<EOT
    #cloud-config
    users:
      - name: ${var.do_droplet_user}
        ssh-authorized-keys:
          - ${var.do_ssh_pub_key}
        sudo: ['ALL=(ALL) NOPASSWD:ALL']
        groups: sudo
        shell: /bin/bash
    write_files:
    - content: |
        [Unit]
        Description=Algeria Startup Jobs API
        After=network.target

        [Service]
        ExecStart=/home/${var.do_droplet_user}/${local.app_folder_name}/${local.app_name}
        Restart=always
        RestartSec=5
        User=${var.do_droplet_user}

        [Install]
        WantedBy=multi-user.target
      path: /etc/systemd/system/${local.service_name}.service
    runcmd:
      - sudo apt update
      - sudo apt install nginx -y
      - sudo ufw allow 'Nginx HTTP'
      - sudo sh -c "echo '
          server {
              listen 80;
              server_name ${local.domain_name};

              location / {
                  proxy_pass http://localhost:9090;
                  proxy_set_header Host \$host;
                  proxy_set_header X-Real-IP \$remote_addr;
              }
          }' > /etc/nginx/sites-available/${local.domain_name}.conf"
      - sudo rm /etc/nginx/sites-enabled/*
      - sudo ln -s /etc/nginx/sites-available/${local.domain_name}.conf /etc/nginx/sites-enabled/
      - sudo systemctl enable nginx
      - sudo systemctl start nginx
      - sudo systemctl daemon-reload
      - systemctl start ${local.service_name}
    EOT
}

resource "digitalocean_project_resources" "api" {
  project = data.terraform_remote_state.shared.outputs.digitalocean_project_id
  resources = [
    digitalocean_droplet.api.urn
  ]
}

resource "aws_route53_record" "api" {
  zone_id = data.terraform_remote_state.shared.outputs.route53_zone_id
  name    = local.domain_name
  type    = "A"
  ttl     = "300"
  records = [digitalocean_droplet.api.ipv4_address]
}

resource "ssh_resource" "always_run" {
  triggers = {
    # @TODO-ZM: change to only run when code change
    always_run = "${timestamp()}"
  }

  host        = digitalocean_droplet.api.ipv4_address
  user        = var.do_droplet_user
  private_key = var.do_ssh_key
  timeout     = "5m"

  file {
    source      = "${path.module}/../../api/ubuntu-target/target/release/${local.app_name}"
    destination = local.upload_tmp_name
  }

  commands = [
    "sudo systemctl stop ${local.service_name} || true",
    "sudo mkdir -p ${local.app_folder} || true",
    "sudo rm ${local.app_folder}/${local.app_name} || true",
    "sudo mv ${local.upload_tmp_name} ${local.app_folder}/${local.app_name} || true",
    "sudo chmod +x ${local.app_folder}/${local.app_name} || true",
    "sudo systemctl start ${local.service_name} || true",
    "sudo systemctl restart nginx"
  ]
}