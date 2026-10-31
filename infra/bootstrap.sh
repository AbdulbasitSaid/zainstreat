#!/bin/bash
# Lightsail launch script (user-data) for Amazon Linux 2023 — Phase 21.
# Reproduces the Phase 2/18 manual server setup so a new box (staging, or
# a rebuilt/moved production) needs no SSH: Docker + Compose, swap,
# persistent journal, the sudo-less `deploy` user and /opt/zainstreat.
#
# A template: workflows replace __DEPLOY_SSH_PUBLIC_KEY__ before passing it
# as --user-data. Idempotent, and contains no secrets.
set -euxo pipefail

# Phase 21 Decision 18: a log the deploy user can read, and a failure
# marker _deploy.yml's wait loop stops on instead of timing out.
mkdir -p /opt/zainstreat
exec > >(tee -a /var/log/zainstreat-bootstrap.log) 2>&1
trap 'echo "bootstrap failed at line $LINENO" > /opt/zainstreat/.bootstrap-failed' ERR

COMPOSE_VERSION=v2.40.0

dnf update -y
# AL2023's own docker package; Docker's repos don't publish for AL2023.
# The Compose v2 plugin isn't packaged, so it's installed as a binary
# (v2.24.4+ is needed for the !reset/!override tags in
# docker-compose.prod.yml).
dnf install -y docker
mkdir -p /usr/local/lib/docker/cli-plugins
curl -fsSL "https://github.com/docker/compose/releases/download/${COMPOSE_VERSION}/docker-compose-linux-$(uname -m)" \
  -o /usr/local/lib/docker/cli-plugins/docker-compose
chmod +x /usr/local/lib/docker/cli-plugins/docker-compose
systemctl enable --now docker

# 2 GB swap, rarely used (Phase 18 Group 3).
if [ ! -f /swapfile ]; then
  fallocate -l 2G /swapfile
  chmod 600 /swapfile
  mkswap /swapfile
  swapon /swapfile
  echo '/swapfile none swap sw 0 0' >> /etc/fstab
fi
echo 'vm.swappiness=10' > /etc/sysctl.d/99-swappiness.conf
sysctl --system

# Persistent journal, so the previous boot's kernel/OOM log survives a
# hang-and-reboot (Phase 18 Group 3).
mkdir -p /etc/systemd/journald.conf.d
printf '[Journal]\nStorage=persistent\nSystemMaxUse=200M\n' > /etc/systemd/journald.conf.d/10-persistent.conf
systemctl restart systemd-journald

# Sudo-less deploy user, docker group only (Phase 2 Group 1b).
id deploy >/dev/null 2>&1 || useradd -m -G docker deploy
install -d -m 700 -o deploy -g deploy /home/deploy/.ssh
echo '__DEPLOY_SSH_PUBLIC_KEY__' > /home/deploy/.ssh/authorized_keys
chown deploy:deploy /home/deploy/.ssh/authorized_keys
chmod 600 /home/deploy/.ssh/authorized_keys

# No git checkout: _deploy.yml uploads the compose files, Caddyfile and a
# generated .env.
install -d -m 750 -o deploy -g deploy /opt/zainstreat /opt/zainstreat/bin
chown deploy:deploy /var/log/zainstreat-bootstrap.log
rm -f /opt/zainstreat/.bootstrap-failed
touch /opt/zainstreat/.bootstrap-done
chown deploy:deploy /opt/zainstreat/.bootstrap-done
