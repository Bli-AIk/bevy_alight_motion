#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  scripts/vast/create_render_template.sh [options]

Options:
  --name <name>            Template name. Default: bevy-alight-motion-render
  --image <image>          Docker image. Default: vastai/base-image
  --image-tag <tag>        Optional image tag.
  --disk-gb <n>            Recommended disk size. Default: 40
  --search-params <query>  Vast offer filters for the template.
  --vast-bin <path>        vastai CLI path.
  --vast-retry <n>         Retry count passed to vastai. Default: 3
  --dry-run                Print request payload without creating the template.
  -h, --help               Show this message.
EOF
}

resolve_vast_bin() {
    if [ -n "${vast_bin:-}" ]; then
        echo "$vast_bin"
        return 0
    fi
    if [ -x "${repo_root}/.venv-vast/bin/vastai" ]; then
        echo "${repo_root}/.venv-vast/bin/vastai"
        return 0
    fi
    command -v vastai
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../.." && pwd)"
onstart_file="${script_dir}/render_onstart.sh"

name="bevy-alight-motion-render"
image="vastai/base-image"
image_tag="@vastai-automatic-tag"
disk_gb=40
search_params="rentable=True rented=False verified=True direct_port_count>=1 num_gpus=1 gpu_ram>=8 disk_space>=40 reliability>0.97"
vast_bin=""
dry_run=0
vast_retry=3

while [ $# -gt 0 ]; do
    case "$1" in
        --name)
            name="${2:-}"
            shift 2
            ;;
        --image)
            image="${2:-}"
            shift 2
            ;;
        --image-tag)
            image_tag="${2:-}"
            shift 2
            ;;
        --disk-gb)
            disk_gb="${2:-}"
            shift 2
            ;;
        --search-params)
            search_params="${2:-}"
            shift 2
            ;;
        --vast-bin)
            vast_bin="${2:-}"
            shift 2
            ;;
        --vast-retry)
            vast_retry="${2:-}"
            shift 2
            ;;
        --dry-run)
            dry_run=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

vast_bin="$(resolve_vast_bin)"
if [ ! -x "$vast_bin" ]; then
    echo "vastai CLI not found: $vast_bin" >&2
    exit 1
fi

[ -f "$onstart_file" ] || {
    echo "Shared onstart script is missing: $onstart_file" >&2
    exit 1
}
onstart_cmd="$(<"$onstart_file")"

cmd=(
    "$vast_bin"
    --retry "$vast_retry"
    create template
    --name "$name"
    --image "$image"
    --disk_space "$disk_gb"
    --search_params "$search_params"
    --ssh
    --direct
    --onstart-cmd "$onstart_cmd"
)

if [ -n "$image_tag" ]; then
    cmd+=(--image_tag "$image_tag")
fi

if [ "$dry_run" -eq 1 ]; then
    printf 'Dry run. Would run: '
    printf '%q ' "${cmd[@]}"
    echo
    printf 'Search params: %s\n' "$search_params"
    printf 'Onstart script:\n%s\n' "$onstart_cmd"
    exit 0
fi

cmd+=(--raw)

printf 'Running: '
printf '%q ' "${cmd[@]}"
echo

output="$("${cmd[@]}")"
printf '%s\n' "$output"

if [ "$dry_run" -eq 0 ]; then
    JSON_INPUT="$output" python3 - <<'PY'
import ast
import json
import os
import re
import sys

raw = os.environ.get("JSON_INPUT", "").strip()
if not raw:
    raise SystemExit(0)

data = None
for chunk in raw.splitlines():
    chunk = chunk.strip()
    if not chunk or chunk == "null":
        continue
    if chunk.startswith("{"):
        try:
            data = json.loads(chunk)
            break
        except json.JSONDecodeError:
            pass
    match = re.match(r"^New Template:\s*(\{.*\})$", chunk)
    if match:
        data = {"template": ast.literal_eval(match.group(1))}
        break

if data is None:
    print(raw)
    raise SystemExit(0)

template = data.get("template", {})
hash_id = template.get("hash_id") or data.get("hash_id")
template_id = template.get("id") or data.get("id")
name = template.get("name") or data.get("name")

print(f"Template created: name={name} id={template_id} hash={hash_id}")
PY
fi
