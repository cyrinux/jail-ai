ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai PHP development environment"

USER root

# Install PHP and common extensions
RUN apt-get update && apt-get install -y --no-install-recommends \
    php8.2 \
    php8.2-cli \
    php8.2-common \
    php8.2-curl \
    php8.2-mbstring \
    php8.2-xml \
    php8.2-zip \
    php8.2-bcmath \
    php8.2-gd \
    php8.2-intl \
    php8.2-mysql \
    php8.2-pgsql \
    php8.2-sqlite3 \
    php8.2-redis \
    && rm -rf /var/lib/apt/lists/* \
    && ln -sf /usr/bin/php8.2 /usr/bin/php

# Install Composer globally
RUN curl -sS https://getcomposer.org/installer | php -- --install-dir=/usr/local/bin --filename=composer

# Install common PHP development tools
USER agent
RUN composer global require --no-cache \
    phpunit/phpunit \
    squizlabs/php_codesniffer \
    phpstan/phpstan \
    friendsofphp/php-cs-fixer

# Add Composer global bin to PATH
ENV PATH="/home/agent/.config/composer/vendor/bin:$PATH"

WORKDIR /workspace

CMD ["/bin/zsh"]
