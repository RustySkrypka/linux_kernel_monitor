#!/usr/bin/bash

# Build project
#cargo build --release

# Get OS release info (using system tools - portable way)
OS_RELEASE=$(cat /etc/os-release)

# Extract the NAME (or ID) field.  This handles variations in /etc/os-release format.
# This will work even if NAME is in quotes, and it's more robust than just grep.
NAME=$(grep "^NAME=" <<< "$OS_RELEASE" | cut -d '=' -f 2 | tr -d '"') # Remove quotes

PROFILE="release"

# Check the distribution
if [[ "$NAME" == "Ubuntu" ]]; then
    echo "Packaging for Ubuntu..."
	cargo deb --profile $PROFILE
	cp -f ./target/debian/linux-kernel-monitor*.deb ./
elif [[ "$NAME" == "RedHat" ]]; then
    echo "Packaging for RedHat..."
	cargo bulid --release
	cargo generate-rpm
	cp -f ./target/generate-rpm/linux-kernel-monitor*.rpm ./
else
    echo "Unsupported distribution: $NAME ($ID)"
    exit 1
fi
