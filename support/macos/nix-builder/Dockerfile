FROM nixos/nix:2.32.8 AS base

ENV PATH=/nix/var/nix/profiles/default/bin:/nix/var/nix/profiles/default/sbin:/usr/local/bin:/usr/bin:/bin

RUN nix-env -iA --profile /nix/var/nix/profiles/default \
        nixpkgs.openssh \
        nixpkgs.python3 \
        nixpkgs.coreutils \
        nixpkgs.nix \
        nixpkgs.tini \
    && mkdir -p /run/sshd /etc/ssh /usr/local/bin

RUN echo 'root:x:0:0:root:/root:/bin/sh' > /etc/passwd \
    && echo 'sshd:x:74:74:Privilege-separated SSH:/var/empty:/sbin/nologin' >> /etc/passwd \
    && echo 'root:x:0:' > /etc/group \
    && echo 'sshd:x:74:' >> /etc/group \
    && echo 'root:*:0:0:99999:7:::' > /etc/shadow \
    && echo 'sshd:!:0:0:99999:7:::' >> /etc/shadow \
    && chmod 640 /etc/shadow \
    && mkdir -p /var/empty /var/lib/nix-builder

COPY setup_nix_conf.py /usr/local/bin/setup_nix_conf.py
RUN python3 /usr/local/bin/setup_nix_conf.py

COPY sshd_config /etc/ssh/sshd_config
COPY entrypoint.py /usr/local/bin/entrypoint.py
RUN chmod +x /usr/local/bin/entrypoint.py

VOLUME ["/var/lib/nix-builder"]

EXPOSE 22

ENTRYPOINT ["tini", "--"]
CMD ["python3", "/usr/local/bin/entrypoint.py"]
