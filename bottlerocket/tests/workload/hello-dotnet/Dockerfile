FROM public.ecr.aws/amazonlinux/amazonlinux:2023

# Install necessary utilities
RUN dnf makecache && \
    dnf install -y \
      gzip \
      tar \
      dotnet \
    && \
    dnf clean all && \
    rm -rf /var/cache/dnf

# Copy the script that will run the application
COPY --chmod=755 ./run.sh /
ENTRYPOINT ["./run.sh"]

