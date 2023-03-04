#!/bin/bash

# Set path to identity file
IDENTITY_FILE=/path/to/identity/file
REMOTE_USER=root
REMOTE_HOST=ohara
REMOTE_DIR=/opt
IDENTITY_FILE=~/.ssh/ff-22.pub
PROJECT_NAME=gamers_bot

# Build Rust project with release flag
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

# Check if yt-dlp file exists on remote host
if ssh -i $IDENTITY_FILE $REMOTE_USER@$REMOTE_HOST "[ -f /opt/gamersbot_release/vendors/yt-dlp_linux ]"; then
   echo "yt-dlp file already exists on remote host"
else
  cp ../vendors/yt-dlp_linux gamersbot_release/vendors/
fi

# Copy gamersbot folder to remote server using identity file
scp -i $IDENTITY_FILE -r gamersbot_release $REMOTE_USER@$REMOTE_HOST:$REMOTE_DIR
ssh -i $IDENTITY_FILE $REMOTE_USER@$REMOTE_HOST "chmod +x /opt/gamersbot_release/vendors/yt-dlp_linux"

#Cleanup Temp Dir
rm -rf gamersbot_release
