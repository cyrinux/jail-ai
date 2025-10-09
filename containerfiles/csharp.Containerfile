ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai C# / .NET development environment"

USER root

# Install .NET SDK (LTS version 8.0)
RUN wget https://packages.microsoft.com/config/debian/12/packages-microsoft-prod.deb -O packages-microsoft-prod.deb \
    && dpkg -i packages-microsoft-prod.deb \
    && rm packages-microsoft-prod.deb \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
    dotnet-sdk-8.0 \
    && rm -rf /var/lib/apt/lists/*

# Set environment variables for .NET
ENV DOTNET_ROOT=/usr/share/dotnet \
    DOTNET_CLI_TELEMETRY_OPTOUT=1 \
    DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1

USER agent

# Install common .NET global tools
RUN dotnet tool install --global dotnet-format \
    && dotnet tool install --global dotnet-ef

# Add .NET tools to PATH
ENV PATH="/home/agent/.dotnet/tools:$PATH"

WORKDIR /workspace

CMD ["/bin/zsh"]
