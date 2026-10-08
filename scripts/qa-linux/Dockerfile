ARG BASE=ubuntu:22.04
FROM ${BASE}
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libicu-dev libssl3 libstdc++6 zlib1g libgssapi-krb5-2 libunwind8 libegl1 libgl1 libglx-mesa0 libgl1-mesa-dri libdrm2 libgbm1 libfontconfig1 fonts-dejavu-core libx11-6 libxext6 libxrender1 libxrandr2 libxcursor1 libxi6 libxfixes3 libasound2-dev xvfb xauth procps util-linux \
 && (apt-get install -y --no-install-recommends libfuse2 || apt-get install -y --no-install-recommends libfuse2t64) \
 && rm -rf /var/lib/apt/lists/*
COPY powershell /opt/pwsh
RUN ln -s /opt/pwsh/pwsh /usr/local/bin/pwsh && chmod +x /opt/pwsh/pwsh
WORKDIR /workspace
ENV SLINT_RENDERER=software LIBGL_ALWAYS_SOFTWARE=1
# Deliberately no libmpv or FFmpeg packages: the AppImage must bring those.
ENTRYPOINT ["xvfb-run", "-a", "pwsh", "-NoProfile", "-File", "scripts/test-velopack-upgrade.ps1", "-Platform", "linux-x64"]
