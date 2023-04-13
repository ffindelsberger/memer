#!/bin/bash

# Set path to identity file
REMOTE_USER=root
REMOTE_HOST=ohara
REMOTE_DIR=/opt
IDENTITY_FILE=~/.ssh/ff-scripts
PROJECT_NAME=gamers_bot

# Check for dependencies on remote host
if ! ssh -i $IDENTITY_FILE $REMOTE_USER@$REMOTE_HOST "command -v yt-dlp >/dev/null && command -v ffmpeg >/dev/null"; then
   echo "Required dependencies not found on remote host"
   exit 1
fi

# Build Rust project with release flag for linux target with static linking
if ! TARGET_CC=x86_64-linux-musl-gcc cargo build --release --target=x86_64-unknown-linux-musl; then
  echo "Cargo build failed, exiting..."
  exit 1
fi

# Create gamersbot directory if it doesn't exist
mkdir -p gamersbot_release

# Move binary to gamersbot directory
mv ../target/x86_64-unknown-linux-musl/release/$PROJECT_NAME gamersbot_release

# Create gamersbot/vendors directory if it doesn't exist
mkdir -p gamersbot_release/vendors

# Check if gamersbot service is running on remote host
if ssh -i $IDENTITY_FILE $REMOTE_USER@$REMOTE_HOST "systemctl is-active gamersbot.service >/dev/null 2>&1"; then
   echo "Stopping gamersbot service on remote host"
   ssh -i $IDENTITY_FILE $REMOTE_USER@$REMOTE_HOST "systemctl stop gamersbot.service"
fi

# Copy gamersbot folder to remote server using identity file
scp -i $IDENTITY_FILE -r gamersbot_release $REMOTE_USER@$REMOTE_HOST:$REMOTE_DIR
ssh -i $IDENTITY_FILE $REMOTE_USER@$REMOTE_HOST "chmod +x /opt/gamersbot_release/vendors/yt-dlp_linux"

# Start gamersbot service on remote host
ssh -i $IDENTITY_FILE $REMOTE_USER@$REMOTE_HOST "systemctl start gamersbot.service"

# Check if gamersbot service is running and print status
ssh -i $IDENTITY_FILE $REMOTE_USER@$REMOTE_HOST "systemctl status gamersbot.service"

#Cleanup Temp Dir
rm -rf gamersbot_release
