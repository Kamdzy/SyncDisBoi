#!/usr/bin/env bash

# Determine the config directory
CONFIG_DIR=${CONFIG_DIR:-$HOME/.config/SyncDisBoi}

# Optional
# Dynamically load the env variables from the args.ini file
if [ -f "$CONFIG_DIR/args.ini" ]; then
    source "$CONFIG_DIR/args.ini"
fi

# Run the script using env variables if they're set either from args.ini or from the docker run command
if [ -n "$SRC_PLATFORM" ] && [ -n "$DST_PLATFORM" ]; then
    echo "Executing: /usr/local/bin/sync_dis_boi $SRC_PLATFORM $DST_PLATFORM $@"
    exec /usr/local/bin/sync_dis_boi $SRC_PLATFORM $DST_PLATFORM $@
    exit 0
fi

# Pass the arguments as extra parameters from docker run if the env variables are not set
echo "Executing: /usr/local/bin/sync_dis_boi $@"
exec /usr/local/bin/sync_dis_boi $@