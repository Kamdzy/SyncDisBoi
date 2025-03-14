#!/usr/bin/env bash

# Determine the config directory
CONFIG_DIR=${CONFIG_DIR:-$HOME/.config/SyncDisBoi}

# Clear console output
clear

# Optional
# Dynamically load the env variables from the args.ini file
if [ -f "$CONFIG_DIR/args.ini" ]; then
    while IFS='=' read -r key value; do
        # Skip lines that are empty or start with #
        [[ -z "$key" || "$key" =~ ^# ]] && continue

        # Trim possible carriage returns (in case the file is from Windows)
        value=$(echo "$value" | tr -d '\r')

        # Export as environment variable
        export "$key"="$value"
    done < "$CONFIG_DIR/args.ini"
fi

# Run the script using env variables if they are set either from args.ini or from the docker run command
if [ -n "$SRC_PLATFORM" ] && [ -n "$DST_PLATFORM" ]; then
    exec /usr/local/bin/sync_dis_boi $SRC_PLATFORM $DST_PLATFORM "$@"
    exit 0
fi

# Pass the arguments as extra parameters from docker run if the env variables are not set
exec /usr/local/bin/sync_dis_boi "$@"