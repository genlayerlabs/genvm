FROM --platform=linux/amd64 nixos/nix@sha256:42d51ae5882b7d9a6ad3237d356025445dc75903b1bfe4eb007a3426356e6c42

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
