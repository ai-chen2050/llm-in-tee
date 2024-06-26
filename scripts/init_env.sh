#!/usr/bin/env bash

sudo sudo dnf upgrade 
sudo dnf install -y tmux htop openssl-devel perl docker-24.0.5-1.amzn2023.0.3 aws-nitro-enclaves-cli aws-nitro-enclaves-cli-devel
sudo usermod -aG ne ec2-user
sudo usermod -aG docker ec2-user
sudo systemctl restart nitro-enclaves-allocator.service
sudo systemctl restart docker
sudo systemctl enable --now nitro-enclaves-allocator.service
sudo systemctl enable --now docker