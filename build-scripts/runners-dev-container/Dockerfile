FROM --platform=linux/amd64 nixos/nix@sha256:3bb728719e2c4e478df4c50b80f93adbe27d5c561d1417c3a2306eb914d910da

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
