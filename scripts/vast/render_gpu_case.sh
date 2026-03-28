#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  scripts/vast/render_gpu_case.sh <baseline|settle|compat> --pattern <pattern> [render_once options...]

Description:
  Wraps scripts/vast/render_once.sh with GPU-only environment presets so each
  comparison run is reproducible and fully logged.

Cases:
  baseline  Run with NVIDIA GPU forced via Vulkan ICD + adapter name.
  settle    baseline + more conservative comparison settle waits.
  compat    baseline + WGPU_SETTINGS_PRIO=compatibility.

Examples:
  scripts/vast/render_gpu_case.sh baseline --pattern basic/fill/nested_solid_color --single --template-hash <hash>
  scripts/vast/render_gpu_case.sh settle --pattern basic/group/fill/basic --single --instance-id <id>
EOF
}

if [ $# -lt 1 ]; then
    usage >&2
    exit 1
fi

case_name="$1"
shift

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
render_once="${script_dir}/render_once.sh"

[ -x "$render_once" ] || {
    echo "render_once.sh is missing or not executable: $render_once" >&2
    exit 1
}

gpu_adapter_name="${AM_VAST_GPU_ADAPTER_NAME:-NVIDIA}"
nvidia_icd="${AM_VAST_NVIDIA_ICD:-/etc/vulkan/icd.d/nvidia_icd.json}"

remote_env=(
    "WGPU_FORCE_FALLBACK_ADAPTER=0"
    "WGPU_ADAPTER_NAME=${gpu_adapter_name}"
    "VK_ICD_FILENAMES=${nvidia_icd}"
)

case "$case_name" in
    baseline)
        ;;
    settle)
        remote_env+=(
            "COMPARISON_INITIAL_WAIT_FRAMES=20"
            "COMPARISON_RENDER_WAIT_FRAMES=10"
            "COMPARISON_WAIT_FRAMES=6"
            "COMPARISON_PRIME_CAPTURES=3"
        )
        ;;
    compat)
        remote_env+=("WGPU_SETTINGS_PRIO=compatibility")
        ;;
    -h|--help)
        usage
        exit 0
        ;;
    *)
        echo "Unknown case: $case_name" >&2
        usage >&2
        exit 1
        ;;
esac

cmd=("$render_once")
for env_kv in "${remote_env[@]}"; do
    cmd+=(--remote-env "$env_kv")
done
cmd+=("$@")

printf '[vast-gpu-case] case=%s\n' "$case_name"
for env_kv in "${remote_env[@]}"; do
    printf '[vast-gpu-case] remote-env=%s\n' "$env_kv"
done
printf '[vast-gpu-case] command='
printf '%q ' "${cmd[@]}"
echo

"${cmd[@]}"
