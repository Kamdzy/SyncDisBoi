FROM ubuntu:latest

# Copy the artifact
COPY dist/linux-x86_64-sync_dis_boi /usr/local/bin/sync_dis_boi
RUN chmod +x /usr/local/bin/sync_dis_boi

# Copy the entrypoint script
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Run the artifact and pass the arguments
ENTRYPOINT ["/entrypoint.sh"]
CMD []