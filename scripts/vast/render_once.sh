#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage:
  scripts/vast/render_once.sh --pattern <pattern> [options]

Options:
  --pattern <pattern>        Comparison filter.
  --single                   Forward --single to test_comparison.sh.
  --frame-test               Run frame-test mode instead of video comparison.
  --instance-id <id>         Reuse an existing instance.
  --offer-id <id>            Use a specific offer instead of searching.
  --template-hash <hash>     Vast template hash for instance creation.
  --image <image>            Docker image when not using a template.
  --onstart-file <path>      Local onstart script for image-based instance creation.
  --no-onstart               Do not send an onstart script with image-based creation.
  --disk-gb <n>              Requested disk size. Default: 40
  --gpu-ram-min <gb>         Offer filter. Default: 8
  --gpu-name <name>          Restrict search to one GPU model.
  --search <query>           Extra Vast offer filters appended to the default query.
  --order <fields>           Vast search ordering. Default: dph,total_flops-
  --remote-root <dir>        Remote bundle root. Default: /root/bevy_alight_motion_remote
  --bundle-dir <dir>         Reuse a local bundle directory instead of mktemp.
  --player-bin <path>        Prebuilt player binary. Default: target/release/examples/player
  --build-local              Build the player locally before bundling.
  --build-features <list>    Player features for --build-local. Default: video-comparison,headless-render
  --vast-bin <path>          vastai CLI path.
  --vast-retry <n>           Retry count passed to vastai. Default: 3
  --watchdog-secs <n>        Local failsafe destroy timeout for created instances. Default: 5400
  --destroy-mode <mode>      destroy or stop on success. Default: destroy
  --keep-on-failure          Stop instead of destroy when this run fails.
  --label <label>            Instance label. Default: bevy-alight-motion
  --ssh-public-key <path>    Local public key attached to the instance.
  --ssh-wait-secs <n>        SSH readiness timeout. Default: 600
  --remote-env <KEY=VAL>     Export one environment variable for the remote comparison run.
  --concurrent-tag <tag>     Unique tag for concurrent runs. Each tag gets its own managed instance
                             file so multiple render_once.sh can run in parallel without conflict.
  --dry-run                  Prepare bundle and resolve offer, but do not create/run the instance.
  -h, --help                 Show this message.
EOF
}

log() {
    printf '[vast-render] %s\n' "$*"
}

die() {
    printf '[vast-render] %s\n' "$*" >&2
    exit 1
}

require_cmd() {
    local cmd="$1"
    command -v "$cmd" >/dev/null 2>&1 || die "Missing required command: $cmd"
}

quote_remote_command() {
    local quoted=""
    local part
    for part in "$@"; do
        printf -v part '%q' "$part"
        quoted+="${part} "
    done
    printf '%s' "${quoted% }"
}

resolve_vast_bin() {
    if [ -n "$vast_bin" ]; then
        echo "$vast_bin"
        return 0
    fi
    if [ -x "${repo_root}/.venv-vast/bin/vastai" ]; then
        echo "${repo_root}/.venv-vast/bin/vastai"
        return 0
    fi
    command -v vastai
}

resolve_default_player_bin() {
    local metadata_target
    metadata_target="$(
        cargo metadata --format-version=1 2>/dev/null | \
            python3 -c "import json,sys; print(json.load(sys.stdin)['target_directory'])" 2>/dev/null || true
    )"
    if [ -n "$metadata_target" ]; then
        printf '%s' "${metadata_target}/release/examples/player"
        return 0
    fi
    printf '%s' "${repo_root}/target/release/examples/player"
}

vast() {
    "$vast_bin" --retry "$vast_retry" "$@"
}

read_managed_instance_state() {
    if [ -f "$managed_instance_file" ]; then
        head -n 1 "$managed_instance_file" | tr -d '\r\n[:space:]'
    fi
}

reap_stale_managed_instance() {
    local stale_instance_id=""

    stale_instance_id="$(read_managed_instance_state || true)"
    if [ -z "$stale_instance_id" ]; then
        rm -f "$managed_instance_file"
        return 0
    fi

    log "Found stale managed instance ${stale_instance_id}; destroying it before starting a new run"
    vast destroy instance "$stale_instance_id" >/dev/null 2>&1 || true
    rm -f "$managed_instance_file"
}

arm_instance_watchdog() {
    if [ "$created_instance" -ne 1 ] || [ "$watchdog_secs" -le 0 ]; then
        return 0
    fi

    mkdir -p "$(dirname "$managed_instance_file")"
    printf '%s\n' "$instance_id" > "$managed_instance_file"

    nohup env \
        VAST_RENDER_WATCHDOG_SECS="$watchdog_secs" \
        VAST_RENDER_INSTANCE_ID="$instance_id" \
        VAST_RENDER_VAST_BIN="$vast_bin" \
        VAST_RENDER_VAST_RETRY="$vast_retry" \
        VAST_RENDER_MANAGED_STATE_FILE="$managed_instance_file" \
        bash -lc '
            sleep "$VAST_RENDER_WATCHDOG_SECS"
            "$VAST_RENDER_VAST_BIN" --retry "$VAST_RENDER_VAST_RETRY" destroy instance "$VAST_RENDER_INSTANCE_ID" >/dev/null 2>&1 || true
            rm -f "$VAST_RENDER_MANAGED_STATE_FILE"
        ' >/dev/null 2>&1 &
    watchdog_pid="$!"

    log "Armed local watchdog for instance ${instance_id} (${watchdog_secs}s)"
}

disarm_instance_watchdog() {
    if [ -n "${watchdog_pid:-}" ]; then
        kill "$watchdog_pid" >/dev/null 2>&1 || true
        wait "$watchdog_pid" >/dev/null 2>&1 || true
        watchdog_pid=""
    fi

    rm -f "$managed_instance_file"
}

normalize_pattern() {
    local value="$1"
    value="${value#./}"
    value="${value#assets/projects/}"
    value="${value#projects/}"
    printf '%s' "$value"
}

determine_project_sync_root() {
    local normalized
    normalized="$(normalize_pattern "$pattern")"
    local projects_root="${repo_root}/assets/projects"

    if [ -z "$normalized" ]; then
        printf '%s' ""
        return 0
    fi

    if [ -f "${projects_root}/${normalized}.amproj" ] || [ -d "${projects_root}/${normalized}.amproj" ]; then
        dirname "$normalized"
        return 0
    fi
    if [ -d "${projects_root}/${normalized}" ]; then
        printf '%s' "$normalized"
        return 0
    fi

    local candidate="$normalized"
    while [ -n "$candidate" ] && [ "$candidate" != "." ]; do
        if [ -d "${projects_root}/${candidate}" ]; then
            printf '%s' "$candidate"
            return 0
        fi
        if [ -f "${projects_root}/${candidate}.amproj" ]; then
            dirname "$candidate"
            return 0
        fi
        candidate="$(dirname "$candidate")"
        if [ "$candidate" = "." ]; then
            candidate=""
        fi
    done

    printf '%s' ""
}

ensure_local_player() {
    if [ "$build_local" -eq 1 ]; then
        log "Building local player with features: $build_features"
        cargo build -p bevy_alight_motion --example player --features "$build_features" --release
    fi

    if [ ! -x "$player_bin" ]; then
        die "Prebuilt player binary not found: $player_bin"
    fi
}

prepare_bundle() {
    if [ -z "$bundle_dir" ]; then
        bundle_dir="$(mktemp -d "${repo_root}/.vast_bundle.XXXXXX")"
        bundle_dir_is_temp=1
    else
        rm -rf "$bundle_dir"
        mkdir -p "$bundle_dir"
        bundle_dir_is_temp=0
    fi

    local sync_root
    sync_root="$(determine_project_sync_root)"
    local projects_root="${repo_root}/assets/projects"

    log "Preparing runtime bundle at ${bundle_dir}"
    mkdir -p "${bundle_dir}/bin" "${bundle_dir}/assets/projects" "${bundle_dir}/scripts/vast"

    cp "$player_bin" "${bundle_dir}/bin/player"
    cp "${repo_root}/test_comparison.sh" "${bundle_dir}/test_comparison.sh"
    cp "${repo_root}/comparison_config.toml" "${bundle_dir}/comparison_config.toml"
    cp "${repo_root}/scripts/vast/remote_run_comparison.sh" \
        "${bundle_dir}/scripts/vast/remote_run_comparison.sh"
    if [ -f "${repo_root}/test_results.json" ]; then
        cp "${repo_root}/test_results.json" "${bundle_dir}/test_results.json"
    fi

    rsync -a "${repo_root}/assets/fonts/" "${bundle_dir}/assets/fonts/"
    rsync -a "${repo_root}/assets/shaders/" "${bundle_dir}/assets/shaders/"

    if [ -n "$sync_root" ]; then
        mkdir -p "${bundle_dir}/assets/projects/${sync_root}"
        rsync -a "${projects_root}/${sync_root}/" "${bundle_dir}/assets/projects/${sync_root}/"
    else
        rsync -a "${projects_root}/" "${bundle_dir}/assets/projects/"
    fi

    chmod +x "${bundle_dir}/test_comparison.sh" "${bundle_dir}/scripts/vast/remote_run_comparison.sh"
}

default_search_query() {
    local query="rentable=True rented=False verified=True direct_port_count>=1 num_gpus=1 gpu_ram>=${gpu_ram_min} disk_space>=${disk_gb} reliability>0.97"
    if [ -n "$gpu_name" ]; then
        query="${query} gpu_name=${gpu_name}"
    fi
    if [ -n "$search_query" ]; then
        query="${query} ${search_query}"
    fi
    printf '%s' "$query"
}

pick_offer() {
    if [ -n "$offer_id" ]; then
        log "Using explicit offer: ${offer_id}"
        return 0
    fi

    local query
    query="$(default_search_query)"
    log "Searching Vast offers with query: ${query}"

    local output
    output="$(vast search offers --raw --limit 1 -o "$order" "$query")"
    offer_id="$(JSON_INPUT="$output" python3 - <<'PY'
import json
import os
import sys

offers = json.loads(os.environ["JSON_INPUT"])
if not offers:
    raise SystemExit(1)
offer = offers[0]
print(offer["id"])
PY
)" || die "No Vast offer matched the query"

    JSON_INPUT="$output" python3 - <<'PY'
import json
import os
import sys

offers = json.loads(os.environ["JSON_INPUT"])
offer = offers[0]
price = offer.get("dph_total", offer.get("search", {}).get("totalHour"))
print(
    "[vast-render] Selected offer id={id} gpu={gpu} gpu_ram={ram:.1f}GB price={price} reliability={rel:.4f} location={loc}".format(
        id=offer["id"],
        gpu=offer.get("gpu_name"),
        ram=float(offer.get("gpu_ram", 0)) / 1024.0,
        price=price,
        rel=float(offer.get("reliability", 0.0)),
        loc=offer.get("geolocation"),
    )
)
PY
}

create_instance_if_needed() {
    if [ -n "$instance_id" ]; then
        created_instance=0
        log "Reusing existing instance: ${instance_id}"
        return 0
    fi

    if [ -z "$template_hash" ] && [ -z "$image" ]; then
        die "Creating a new instance requires --template-hash or --image"
    fi

    local cmd=(
        vast
        create instance
        "$offer_id"
        --raw
        --disk "$disk_gb"
        --ssh
        --direct
        --cancel-unavail
        --label "$label"
    )

    if [ -n "$template_hash" ]; then
        cmd+=(--template_hash "$template_hash")
    fi
    if [ -n "$image" ]; then
        cmd+=(--image "$image")
    fi
    if [ -n "$onstart_file" ]; then
        cmd+=(--onstart "$onstart_file")
    fi

    log "Creating instance from offer ${offer_id}"
    local output
    output="$("${cmd[@]}" 2>&1)"

    if ! instance_id="$(JSON_INPUT="$output" python3 - <<'PY'
import json
import os
import sys

raw = os.environ["JSON_INPUT"].strip()
if not raw:
    raise SystemExit(1)

candidates = [raw]
json_start = min(
    [idx for idx in (raw.find("{"), raw.find("[")) if idx != -1],
    default=-1,
)
if json_start != -1:
    json_tail = raw[json_start:]
    candidates.append(json_tail)
    for end_char in ("}", "]"):
        json_end = json_tail.rfind(end_char)
        if json_end != -1:
            candidates.append(json_tail[: json_end + 1])

data = None
for candidate in candidates:
    try:
        data = json.loads(candidate)
        break
    except json.JSONDecodeError:
        continue

if isinstance(data, list):
    data = data[0] if data else None

if not isinstance(data, dict):
    raise SystemExit(1)

value = data.get("new_contract") or data.get("new_instance") or data.get("id")
if value is None:
    raise SystemExit(1)
print(value)
PY
    )"; then
        printf '[vast-render] Vast create instance raw output:\n%s\n' "$output" >&2
        die "Failed to parse Vast instance id"
    fi

    created_instance=1
    log "Created instance: ${instance_id}"
}

wait_for_ssh() {
    local deadline=$(( $(date +%s) + ssh_wait_secs ))
    local ssh_url_raw=""
    local last_health_check=0
    local proxy_logged=0

    while [ "$(date +%s)" -lt "$deadline" ]; do
        ssh_url_raw="$(vast ssh-url "$instance_id" 2>/dev/null || true)"
        ssh_url_raw="$(printf '%s' "$ssh_url_raw" | tr -d '\r' | tail -n 1)"

        if [[ "$ssh_url_raw" == ssh://* ]]; then
            ssh_user_host="${ssh_url_raw#ssh://}"
            ssh_port="${ssh_user_host##*:}"
            ssh_user_host="${ssh_user_host%:*}"

            if [[ "$ssh_user_host" == *.vast.ai ]] && [ "$proxy_logged" -eq 0 ]; then
                log "Proxy SSH endpoint detected: ${ssh_user_host}:${ssh_port}"
                proxy_logged=1
            fi

            if ssh \
                -o BatchMode=yes \
                -o StrictHostKeyChecking=no \
                -o UserKnownHostsFile=/dev/null \
                -o ConnectTimeout=10 \
                -p "$ssh_port" \
                "$ssh_user_host" \
                true >/dev/null 2>&1; then
                log "SSH is ready at ${ssh_user_host}:${ssh_port}"
                return 0
            fi
        fi

        if [ $(( $(date +%s) - last_health_check )) -ge 30 ]; then
            local health_json
            health_json="$(timeout 20 "$vast_bin" --retry "$vast_retry" show instance "$instance_id" --raw 2>/dev/null || true)"
            if [ -n "$health_json" ]; then
                JSON_INPUT="$health_json" python3 - <<'PY'
import json
import os
import sys

data = json.loads(os.environ["JSON_INPUT"])
actual = (
    data.get("actual_status")
    or data.get("cur_state")
    or data.get("intended_status")
    or ""
)
message = (data.get("status_msg") or "").strip()

if message:
    print(f"[vast-render] Instance status: {actual} msg={message}")
    lowered = message.lower()
    fatal_markers = (
        "manifest unknown",
        "no such container",
        "error response from daemon",
    )
    if any(marker in lowered for marker in fatal_markers):
        raise SystemExit(42)
else:
    print(f"[vast-render] Instance status: {actual}")
PY
                case $? in
                    0)
                        ;;
                    42)
                        die "Remote instance became unhealthy while waiting for SSH"
                        ;;
                    *)
                        ;;
                esac
            fi
            last_health_check="$(date +%s)"
        fi

        sleep 5
    done

    die "Timed out waiting for SSH on instance ${instance_id}"
}

wait_for_remote_runtime() {
    local deadline=$(( $(date +%s) + ssh_wait_secs ))
    local probe_cmd
    probe_cmd="$(quote_remote_command bash -lc \
        "command -v ffmpeg >/dev/null 2>&1 && \
         command -v ffprobe >/dev/null 2>&1 && \
         command -v bc >/dev/null 2>&1 && \
         command -v rsync >/dev/null 2>&1 && \
         command -v python3 >/dev/null 2>&1 && \
         command -v vulkaninfo >/dev/null 2>&1")"

    while [ "$(date +%s)" -lt "$deadline" ]; do
        if ssh "${ssh_common[@]}" -p "$ssh_port" "$ssh_user_host" "$probe_cmd" >/dev/null 2>&1; then
            log "Remote runtime dependencies are ready"
            return 0
        fi
        log "Waiting for remote runtime dependencies to finish installing"
        sleep 5
    done

    die "Timed out waiting for remote runtime dependencies on instance ${instance_id}"
}

attach_ssh_key_if_available() {
    if [ -z "$ssh_public_key_path" ]; then
        return 0
    fi
    if [ ! -f "$ssh_public_key_path" ]; then
        log "SSH public key file is missing, skipping attach: ${ssh_public_key_path}"
        return 0
    fi

    local ssh_key_content
    ssh_key_content="$(<"$ssh_public_key_path")"
    if [ -z "$ssh_key_content" ]; then
        log "SSH public key file is empty, skipping attach: ${ssh_public_key_path}"
        return 0
    fi

    log "Attaching SSH public key to instance ${instance_id}"
    vast attach ssh "$instance_id" "$ssh_key_content" >/dev/null
}

remote_supports_rsync() {
    local cmd
    cmd="$(quote_remote_command bash -lc "command -v rsync >/dev/null 2>&1")"
    if ssh "${ssh_common[@]}" -p "$ssh_port" "$ssh_user_host" "$cmd" >/dev/null 2>&1; then
        return 0
    fi
    return 1
}

push_bundle() {
    local mkdir_cmd
    mkdir_cmd="$(quote_remote_command mkdir -p "$remote_root")"
    ssh "${ssh_common[@]}" -p "$ssh_port" "$ssh_user_host" "$mkdir_cmd" >/dev/null

    if remote_supports_rsync; then
        log "Uploading bundle with rsync"
        if rsync -az --delete \
            -e "ssh ${ssh_common[*]} -p ${ssh_port}" \
            "${bundle_dir}/" "${ssh_user_host}:${remote_root}/"; then
            return 0
        fi
        log "rsync upload failed, falling back to tar over ssh"
    else
        log "Remote rsync missing, falling back to tar over ssh"
    fi

    local remote_cmd
    remote_cmd="$(quote_remote_command bash -lc "rm -rf \"${remote_root}\" && mkdir -p \"${remote_root}\" && tar -xf - -C \"${remote_root}\"")"
    tar -C "$bundle_dir" -cf - . | ssh "${ssh_common[@]}" -p "$ssh_port" "$ssh_user_host" "$remote_cmd"
}

run_remote_comparison() {
    local remote_script="${remote_root}/scripts/vast/remote_run_comparison.sh"
    local remote_cmd=()
    if [ "${#remote_env[@]}" -gt 0 ]; then
        remote_cmd+=(env "${remote_env[@]}")
    fi
    remote_cmd+=(bash "$remote_script" --workdir "$remote_root" --pattern "$pattern")
    if [ "$exact_match" -eq 1 ]; then
        remote_cmd+=(--single)
    fi
    if [ "$frame_test" -eq 1 ]; then
        remote_cmd+=(--frame-test)
    fi
    # The preflight player probe can create report-dir collisions and wastes GPU wall time.
    remote_cmd+=(--skip-render-probe)

    local quoted
    quoted="$(quote_remote_command "${remote_cmd[@]}")"
    ssh "${ssh_common[@]}" -p "$ssh_port" "$ssh_user_host" "$quoted"
}

pull_results_back() {
    local pull_dir
    pull_dir="$(mktemp -d "${repo_root}/.vast_pull.XXXXXX")"
    pull_dir_is_temp=1
    local attempt

    if remote_supports_rsync; then
        for attempt in 1 2 3; do
            local rsync_ok=true
            # reports/ may not exist in frame-test mode
            rsync -az --partial --append-verify \
                -e "ssh ${ssh_common[*]} -p ${ssh_port}" \
                "${ssh_user_host}:${remote_root}/reports/" "${pull_dir}/reports/" 2>/dev/null || true
            rsync -az --partial --append-verify \
                -e "ssh ${ssh_common[*]} -p ${ssh_port}" \
                "${ssh_user_host}:${remote_root}/logs/" "${pull_dir}/logs/" 2>/dev/null || rsync_ok=false
            # test_results.json may not exist in frame-test mode
            rsync -az --partial --append-verify \
                -e "ssh ${ssh_common[*]} -p ${ssh_port}" \
                "${ssh_user_host}:${remote_root}/test_results.json" "${pull_dir}/test_results.json" 2>/dev/null || true
            # perf_results.json may not exist in comparison mode
            rsync -az --partial --append-verify \
                -e "ssh ${ssh_common[*]} -p ${ssh_port}" \
                "${ssh_user_host}:${remote_root}/perf_results.json" "${pull_dir}/perf_results.json" 2>/dev/null || true

            if [ "$rsync_ok" = true ]; then
                break
            fi

            if [ "$attempt" -eq 3 ]; then
                log "rsync pull failed after ${attempt} attempts, falling back to tar over ssh"
                break
            fi
            log "rsync pull attempt ${attempt} failed, retrying"
            sleep 3
        done
    fi

    if [ ! -d "${pull_dir}/reports" ] && [ ! -f "${pull_dir}/test_results.json" ] && [ ! -f "${pull_dir}/perf_results.json" ]; then
        local remote_cmd
        remote_cmd="$(quote_remote_command bash -lc "cd \"${remote_root}\" && tar --ignore-failed-read -cf - test_results.json perf_results.json reports logs")"
        for attempt in 1 2 3; do
            if ssh "${ssh_common[@]}" -p "$ssh_port" "$ssh_user_host" "$remote_cmd" | tar -xf - -C "$pull_dir"; then
                break
            fi

            if [ "$attempt" -eq 3 ]; then
                die "Failed to pull remote results"
            fi
            log "tar pull attempt ${attempt} failed, retrying"
            sleep 3
        done
    fi

    mkdir -p "${repo_root}/reports" "${repo_root}/logs/vast"
    if [ -d "${pull_dir}/reports" ]; then
        cp -a "${pull_dir}/reports/." "${repo_root}/reports/"
    fi
    if [ -d "${pull_dir}/logs" ]; then
        cp -a "${pull_dir}/logs/." "${repo_root}/logs/vast/"
    fi
    if [ -f "${pull_dir}/test_results.json" ]; then
        cp "${pull_dir}/test_results.json" "${repo_root}/test_results.json"
    fi
    if [ -f "${pull_dir}/perf_results.json" ]; then
        cp "${pull_dir}/perf_results.json" "${repo_root}/perf_results.json"
    fi

    rm -rf "$pull_dir"
    pull_dir_is_temp=0
}

summarize_local_results() {
    python3 - "$repo_root/test_results.json" "$(normalize_pattern "$pattern")" "$exact_match" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
pattern = sys.argv[2]
single = sys.argv[3] == "1"

if not path.exists():
    print("[vast-render] test_results.json is missing after sync")
    raise SystemExit(0)

data = json.loads(path.read_text())
results = data.get("results", {})

if single:
    matched = {name: info for name, info in results.items() if name == pattern}
else:
    matched = {name: info for name, info in results.items() if name.startswith(pattern)}

if not matched:
    print(f"[vast-render] No local results matched pattern: {pattern}")
    raise SystemExit(0)

counts = {"pass": 0, "warning": 0, "skip": 0, "cancelled": 0, "fail": 0, "other": 0}
for info in matched.values():
    status = info.get("status", "other")
    counts[status if status in counts else "other"] += 1

print(
    "[vast-render] Summary matched={matched} pass={pass_} warning={warning} skip={skip} cancelled={cancelled} fail={fail}".format(
        matched=len(matched),
        pass_=counts["pass"],
        warning=counts["warning"],
        skip=counts["skip"],
        cancelled=counts["cancelled"],
        fail=counts["fail"],
    )
)
PY
}

summarize_perf_results() {
    python3 - "$repo_root/perf_results.json" "$(normalize_pattern "$pattern")" "$exact_match" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
pattern = sys.argv[2]
single = sys.argv[3] == "1"

if not path.exists():
    print("[vast-render] perf_results.json is missing after sync")
    raise SystemExit(0)

data = json.loads(path.read_text())
results = data.get("results", {})

if single:
    matched = {name: info for name, info in results.items() if name == pattern}
else:
    matched = {name: info for name, info in results.items() if name.startswith(pattern)}

if not matched:
    print(f"[vast-render] No perf results matched pattern: {pattern}")
    raise SystemExit(0)

counts = {"pass": 0, "warning": 0, "fail": 0, "other": 0}
for name, info in matched.items():
    status = info.get("status", "other")
    counts[status if status in counts else "other"] += 1
    avg_fps = info.get("avg_fps", "?")
    p99_fps = info.get("p99_fps", "?")
    stutter = info.get("stutter_count", 0)
    stutter_rate = info.get("stutter_rate", 0)
    max_ft = info.get("max_frame_time_ms", "?")
    mode = info.get("mode", "?")
    print(f"  {name}: {status} | avg={avg_fps:.1f} p99={p99_fps:.1f} fps | stutters={stutter} ({stutter_rate:.1%}) | max_ft={max_ft:.1f}ms | mode={mode}")

print(
    "[vast-render] Perf summary matched={matched} pass={pass_} warning={warning} fail={fail}".format(
        matched=len(matched),
        pass_=counts["pass"],
        warning=counts["warning"],
        fail=counts["fail"],
    )
)
PY
}

teardown_created_instance() {
    local mode="$1"
    if [ "$created_instance" -ne 1 ]; then
        return 0
    fi

    case "$mode" in
        destroy)
            log "Destroying instance ${instance_id}"
            vast destroy instance "$instance_id" >/dev/null
            ;;
        stop)
            log "Stopping instance ${instance_id}"
            vast stop instance "$instance_id" >/dev/null
            ;;
        *)
            die "Unsupported destroy mode: ${mode}"
            ;;
    esac
}

cleanup() {
    local status="${1:-$?}"
    local final_mode=""
    local teardown_succeeded=0

    if [ "$created_instance" -eq 1 ]; then
        if [ "$status" -eq 0 ]; then
            final_mode="$destroy_mode"
        elif [ "$keep_on_failure" -eq 1 ]; then
            log "Run failed; preserving instance ${instance_id} via stop"
            final_mode="stop"
        else
            final_mode="destroy"
        fi

        if [ -n "$final_mode" ] && teardown_created_instance "$final_mode"; then
            teardown_succeeded=1
        fi

        if [ "$final_mode" = "destroy" ] && [ "$teardown_succeeded" -eq 1 ]; then
            disarm_instance_watchdog
        elif [ "$final_mode" = "stop" ]; then
            log "Instance ${instance_id} was stopped; watchdog remains armed for ${watchdog_secs}s as a cost failsafe"
        else
            log "Instance ${instance_id} destroy path was not confirmed; watchdog remains armed for ${watchdog_secs}s"
        fi
    fi

    if [ "${bundle_dir_is_temp:-0}" -eq 1 ] && [ -n "${bundle_dir:-}" ] && [ -d "$bundle_dir" ]; then
        rm -rf "$bundle_dir"
    fi
}

handle_signal() {
    local signal="$1"
    local status=1

    case "$signal" in
        HUP)
            status=129
            ;;
        INT)
            status=130
            ;;
        TERM)
            status=143
            ;;
    esac

    log "Received ${signal}; forcing managed instance teardown"
    trap - EXIT INT TERM HUP
    cleanup "$status"
    exit "$status"
}

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../.." && pwd)"

pattern=""
exact_match=0
instance_id=""
offer_id=""
template_hash=""
image=""
disk_gb=40
gpu_ram_min=8
gpu_name=""
search_query=""
order="dph,total_flops-"
remote_root="/root/bevy_alight_motion_remote"
bundle_dir=""
bundle_dir_is_temp=0
player_bin=""
build_local=0
build_features="video-comparison,headless-render"
frame_test=0
vast_bin=""
vast_retry=3
watchdog_secs=5400
destroy_mode="destroy"
keep_on_failure=0
label="bevy-alight-motion"
ssh_wait_secs=600
dry_run=0
created_instance=0
ssh_user_host=""
ssh_port=""
managed_instance_file=""
watchdog_pid=""
ssh_common=(-o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null)
onstart_file="${script_dir}/render_onstart.sh"
ssh_public_key_path="/root/.ssh/id_ed25519.pub"
remote_env=()
concurrent_tag=""

while [ $# -gt 0 ]; do
    case "$1" in
        --pattern)
            pattern="${2:-}"
            shift 2
            ;;
        --single)
            exact_match=1
            shift
            ;;
        --frame-test)
            frame_test=1
            build_features="frame-test,headless-render"
            shift
            ;;
        --instance-id)
            instance_id="${2:-}"
            shift 2
            ;;
        --offer-id)
            offer_id="${2:-}"
            shift 2
            ;;
        --template-hash)
            template_hash="${2:-}"
            shift 2
            ;;
        --image)
            image="${2:-}"
            shift 2
            ;;
        --onstart-file)
            onstart_file="${2:-}"
            shift 2
            ;;
        --no-onstart)
            onstart_file=""
            shift
            ;;
        --disk-gb)
            disk_gb="${2:-}"
            shift 2
            ;;
        --gpu-ram-min)
            gpu_ram_min="${2:-}"
            shift 2
            ;;
        --gpu-name)
            gpu_name="${2:-}"
            shift 2
            ;;
        --search)
            search_query="${2:-}"
            shift 2
            ;;
        --order)
            order="${2:-}"
            shift 2
            ;;
        --remote-root)
            remote_root="${2:-}"
            shift 2
            ;;
        --bundle-dir)
            bundle_dir="${2:-}"
            shift 2
            ;;
        --player-bin)
            player_bin="${2:-}"
            shift 2
            ;;
        --build-local)
            build_local=1
            shift
            ;;
        --build-features)
            build_features="${2:-}"
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
        --watchdog-secs)
            watchdog_secs="${2:-}"
            shift 2
            ;;
        --destroy-mode)
            destroy_mode="${2:-}"
            shift 2
            ;;
        --keep-on-failure)
            keep_on_failure=1
            shift
            ;;
        --label)
            label="${2:-}"
            shift 2
            ;;
        --ssh-public-key)
            ssh_public_key_path="${2:-}"
            shift 2
            ;;
        --ssh-wait-secs)
            ssh_wait_secs="${2:-}"
            shift 2
            ;;
        --remote-env)
            remote_env+=("${2:-}")
            shift 2
            ;;
        --concurrent-tag)
            concurrent_tag="${2:-}"
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

[ -n "$pattern" ] || die "--pattern is required"
[ "$destroy_mode" = "destroy" ] || [ "$destroy_mode" = "stop" ] || die "--destroy-mode must be destroy or stop"
[[ "$watchdog_secs" =~ ^[0-9]+$ ]] || die "--watchdog-secs must be a non-negative integer"

vast_bin="$(resolve_vast_bin)"
[ -x "$vast_bin" ] || die "vastai CLI not found: ${vast_bin}"

require_cmd python3
require_cmd rsync
require_cmd ssh
require_cmd tar

cd "$repo_root"

if [ -z "$player_bin" ]; then
    player_bin="$(resolve_default_player_bin)"
fi

if [ -n "$onstart_file" ]; then
    onstart_file="$(cd "$(dirname "$onstart_file")" && pwd)/$(basename "$onstart_file")"
fi

if [ -n "$concurrent_tag" ]; then
    managed_instance_file="/root/.cache/vastai/bevy_alight_motion_managed_instance_${concurrent_tag}"
else
    managed_instance_file="/root/.cache/vastai/bevy_alight_motion_managed_instance"
fi

trap 'cleanup $?' EXIT
trap 'handle_signal HUP' HUP
trap 'handle_signal INT' INT
trap 'handle_signal TERM' TERM

ensure_local_player
prepare_bundle
pick_offer

if [ "$dry_run" -eq 1 ]; then
    log "Dry run complete. Bundle=${bundle_dir} offer=${offer_id} template_hash=${template_hash:-<none>} image=${image:-<none>}"
    exit 0
fi

if [ -z "$instance_id" ]; then
    reap_stale_managed_instance
fi

create_instance_if_needed
arm_instance_watchdog
attach_ssh_key_if_available
wait_for_ssh
wait_for_remote_runtime
push_bundle
remote_status=0
if run_remote_comparison; then
    remote_status=0
else
    remote_status=$?
fi
pull_results_back
if [ "$frame_test" -eq 1 ]; then
    summarize_perf_results
else
    summarize_local_results
fi
exit "$remote_status"
