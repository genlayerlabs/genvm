FROM debian:12.10

ENV PATH="/extra_path:${PATH}"
ENV DEBIAN_FRONTEND=noninteractive
ENV PUPPETEER_SKIP_CHROMIUM_DOWNLOAD=true
ENV PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium

SHELL ["/bin/bash", "-c"]

RUN \
    set -ex && \
    uname -m ; \
    mkdir -p /extra_path/ ; \
    mkdir -p /wd/ ; \
    apt-get -y update ; \
    apt-get install -y --no-install-recommends \
        python3 python3-aiohttp \
        unzip \
        curl \
        gnupg \
        ca-certificates \
        fonts-liberation \
        libappindicator3-1 \
        libasound2 \
        libatk-bridge2.0-0 \
        libatk1.0-0 \
        libcups2 \
        libdbus-1-3 \
        libdrm2 \
        libgtk-3-0 \
        libnspr4 \
        libnss3 \
        libx11-xcb1 \
        libxcomposite1 \
        libxdamage1 \
        libxrandr2 \
        xdg-utils \
        chromium \
        chromium-sandbox ; \
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash - ; \
    apt-get install -y nodejs ; \
    apt-get clean ; \
    rm -rf /var/lib/apt/lists/* ; \
    true

HEALTHCHECK --interval=5s --timeout=5s --retries=10 --start-period=3s CMD ["python3", "/src/healthcheck.py"]

COPY ./src /src

RUN \
    set -ex && \
    which chromium ; \
    cd /src/prj && npm ci ; \
    npm run build ; \
    true

# python3 /src/test-install.py ; \

RUN useradd -m appuser && chown -R appuser /wd
USER appuser

WORKDIR /wd

CMD ["bash", "/src/start.sh"]
