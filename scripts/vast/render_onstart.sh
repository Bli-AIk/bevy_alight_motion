set -euo pipefail
export DEBIAN_FRONTEND=noninteractive

need_packages=0
for cmd in ffmpeg ffprobe bc rsync python3 vulkaninfo; do
    if ! command -v "$cmd" >/dev/null 2>&1; then
        need_packages=1
        break
    fi
done

if [ "$need_packages" -eq 1 ]; then
    if ! command -v apt-get >/dev/null 2>&1; then
        echo "apt-get is unavailable; install ffmpeg ffprobe bc rsync python3 manually." >&2
        exit 1
    fi

    apt-get update
    # libasound2 was renamed libasound2t64 in Ubuntu 24.04+
    alsa_pkg="libasound2"
    if ! apt-cache show libasound2 >/dev/null 2>&1; then
        alsa_pkg="libasound2t64"
    fi
    apt-get install -y --no-install-recommends \
        bc \
        ca-certificates \
        ffmpeg \
        "$alsa_pkg" \
        libgl1 \
        libvulkan1 \
        libudev1 \
        mesa-vulkan-drivers \
        python3 \
        python3-pip \
        rsync \
        vulkan-tools
fi

# Ensure tomli is available for Python < 3.11 (Ubuntu 22.04 ships 3.10)
python3 -c "import tomllib" 2>/dev/null || python3 -c "import tomli" 2>/dev/null || \
    pip3 install --quiet --break-system-packages tomli 2>/dev/null || \
    pip3 install --quiet tomli 2>/dev/null || true

mkdir -p /workspace
