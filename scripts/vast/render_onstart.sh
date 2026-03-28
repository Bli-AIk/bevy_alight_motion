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
    apt-get install -y --no-install-recommends \
        bc \
        ca-certificates \
        ffmpeg \
        libasound2 \
        libgl1 \
        libvulkan1 \
        libudev1 \
        mesa-vulkan-drivers \
        python3 \
        rsync \
        vulkan-tools
fi

mkdir -p /workspace
