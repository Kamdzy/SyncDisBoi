#!/usr/bin/env bash

# Optional
# Dynamically load the env variables from the args.ini file
if [ -f "$HOME/.config/SyncDisBoi/args.ini" ]; then
    source "$HOME/.config/SyncDisBoi/args.ini"
fi

# Run the script using env variables if they're set either from args.ini or from the docker run command
if [ -n "$SRC_PLATFORM" ] && [ -n "$DST_PLATFORM" ]; then
    exec /usr/local/bin/sync_dis_boi "$SRC_PLATFORM" "$DST_PLATFORM" "$@"
    exit 0
fi

# Pass the arguments as extra parameters from docker run if the env variables are not set
exec /usr/local/bin/sync_dis_boi "$@"