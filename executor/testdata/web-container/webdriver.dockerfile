FROM --platform=linux/amd64 ubuntu:22.04

WORKDIR /driver

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        python3 unzip curl \
        ca-certificates fonts-liberation libasound2 libatk-bridge2.0-0 libatk1.0-0 libc6 libcairo2 libcups2 libdbus-1-3 libexpat1 libfontconfig1 libgbm1 libgcc1 libglib2.0-0 libgtk-3-0 libnspr4 libnss3 libpango-1.0-0 libpangocairo-1.0-0 libstdc++6 libx11-6 libx11-xcb1 libxcb1 libxcomposite1 libxcursor1 libxdamage1 libxext6 libxfixes3 libxi6 libxrandr2 libxrender1 libxss1 libxtst6 lsb-release xdg-utils && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

ADD --checksum=sha256:2e2e1fdfecd99761c26f1c08c8ba729512753cc45b5772ca5bb7411272b221da \
    https://storage.googleapis.com/chrome-for-testing-public/128.0.6613.119/linux64/chromedriver-linux64.zip \
    /driver/chromedriver.zip

ADD --checksum=sha256:6ba5922ffe3dc0184ec5c1a62b6fb414ff2c1bd4b1efcf1fa9614977c56ea199 \
    https://storage.googleapis.com/chrome-for-testing-public/128.0.6613.119/linux64/chrome-linux64.zip \
    /driver/chrome.zip

RUN \
    unzip chromedriver.zip && \
    unzip chrome.zip && \
    rm chromedriver.zip chrome.zip

ENV PATH="/driver/chromedriver-linux64/:/driver/chrome-linux64/:${PATH}"

RUN useradd -m appuser && chown -R appuser:appuser /driver

USER appuser
EXPOSE 4444

COPY entry.sh /driver/entry.sh
COPY server.py /driver/server.py

CMD ["/driver/entry.sh"]
