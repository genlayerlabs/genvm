FROM debian:12.10

ENV PATH="/extra_path:${PATH}"
ENV DEBIAN_FRONTEND=noninteractive

SHELL ["/bin/bash", "-c"]

RUN \
    set -ex && \
    uname -m ; \
    mkdir -p /download/ ; \
    mkdir -p /extra_path/ ; \
    mkdir -p /wd/ ; \
    apt-get -y update ; \
    apt-get install -y --no-install-recommends \
        python3 python3-aiohttp \
        unzip \
        openjdk-17-jre-headless \
        firefox-esr ; \
    apt-get clean ; \
    rm -rf /var/lib/apt/lists/* ; \
    true

HEALTHCHECK --interval=5s --timeout=5s --retries=10 --start-period=3s CMD ["python3", "/src/healthcheck.py"]

ADD --checksum=sha256:b5fd13fc8f05ca99292700896d43fa986747a01b5fdc77f7bfc88a3707dc4241 \
    https://github.com/mozilla/geckodriver/releases/download/v0.36.0/geckodriver-v0.36.0-linux-aarch64.tar.gz \
    /download/geckodriver-aarch64.tar.gz

ADD --checksum=sha256:0bde38707eb0a686a20c6bd50f4adcc7d60d4f73c60eb83ee9e0db8f65823e04 \
    https://github.com/mozilla/geckodriver/releases/download/v0.36.0/geckodriver-v0.36.0-linux64.tar.gz \
    /download/geckodriver-amd64.tar.gz

ADD --checksum=sha256:aa3bd6e9820a359a54da0cf06836ad42a7555ee13b5235ff60228f637db9bb57 \
    https://github.com/SeleniumHQ/selenium/releases/download/selenium-4.24.0/selenium-server-4.24.0.jar \
    /download/selenium-server.jar

COPY ./src /src

RUN \
    set -ex && \
    pushd /download ; \
    if uname -m | grep -P 'arm|aarch' ; then \
        tar -xf geckodriver-aarch64.tar.gz ; \
    else \
        tar -xf geckodriver-amd64.tar.gz ; \
    fi ; \
    mv geckodriver /extra_path/ ; \
    mv selenium-server.jar /wd ; \
    rm *.tar.gz ; \
    popd ; \
    geckodriver --version ; \
    which firefox ; \
    python3 /src/test-install.py ; \
    true

RUN useradd -m appuser && chown -R appuser /wd
USER appuser

WORKDIR /wd

CMD ["bash", "/src/start.sh"]
