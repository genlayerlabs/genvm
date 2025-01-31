FROM --platform=linux/amd64 nixos/nix@sha256:4bfe741027f0bfbef745862cbf3b5f424ee99801c04138146c0000a28759636d

RUN \
	nix-channel --list && \
	nix-channel --remove nixpkgs && \
	echo 'experimental-features = nix-command flakes' >> /etc/nix/nix.conf && \
	mkdir -p /profiles && \
	mkdir -p /out

RUN --mount=type=bind,source=nix,target=/config \
	cd /config && \
	for i in genvmMain ; \
	do \
		(nix develop --profile "/profiles/$i" ".#$i" -c "true" || exit "1") ; \
	done
