#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ./scripts/docker-build.sh [options]

Build options:
  -i, --image NAME         Image name (default: repopulse)
  -t, --tag TAG            Image tag  (default: local)
  -f, --file PATH          Dockerfile path (default: ./Dockerfile)
  -C, --context PATH       Build context (default: repo root)
      --target NAME        Build a specific target stage
      --platform LIST      Platforms, e.g. linux/amd64 or linux/amd64,linux/arm64
      --build-arg K=V      Pass build-arg (repeatable)
      --no-cache           Disable build cache
      --pull               Always attempt to pull newer base images
      --progress MODE      auto|plain|tty (default: auto)

Buildx output options:
      --push               Push to registry (requires --image to include registry if needed)
      --load               Load image into local docker after buildx build

Other:
  -h, --help               Show this help

Examples:
  ./scripts/docker-build.sh
  ./scripts/docker-build.sh --tag dev
  ./scripts/docker-build.sh --image ghcr.io/you/repopulse --tag v0.1.0 --push
  ./scripts/docker-build.sh --platform linux/amd64,linux/arm64 --tag v0.1.0 --push
EOF
}

IMAGE="repopulse"
TAG="local"
DOCKERFILE="./Dockerfile"
CONTEXT=""
TARGET=""
PLATFORM=""
NO_CACHE="false"
PULL="false"
PROGRESS="auto"
PUSH="false"
LOAD="false"

BUILD_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -i|--image) IMAGE="${2:-}"; shift 2 ;;
    -t|--tag) TAG="${2:-}"; shift 2 ;;
    -f|--file) DOCKERFILE="${2:-}"; shift 2 ;;
    -C|--context) CONTEXT="${2:-}"; shift 2 ;;
    --target) TARGET="${2:-}"; shift 2 ;;
    --platform) PLATFORM="${2:-}"; shift 2 ;;
    --build-arg) BUILD_ARGS+=("$1" "${2:-}"); shift 2 ;;
    --no-cache) NO_CACHE="true"; shift ;;
    --pull) PULL="true"; shift ;;
    --progress) PROGRESS="${2:-}"; shift 2 ;;
    --push) PUSH="true"; shift ;;
    --load) LOAD="true"; shift ;;
    -h|--help) usage; exit 0 ;;
    *)
      echo "Unknown option: $1" >&2
      echo >&2
      usage >&2
      exit 2
      ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [[ -z "$CONTEXT" ]]; then
  CONTEXT="$REPO_ROOT"
fi

DOCKERFILE_PATH="$DOCKERFILE"
if [[ "$DOCKERFILE_PATH" != /* ]]; then
  DOCKERFILE_PATH="$REPO_ROOT/$DOCKERFILE_PATH"
fi

if [[ ! -f "$DOCKERFILE_PATH" ]]; then
  echo "Dockerfile not found: $DOCKERFILE_PATH" >&2
  exit 1
fi

FULL_TAG="${IMAGE}:${TAG}"

COMMON_ARGS=(--file "$DOCKERFILE_PATH" --progress "$PROGRESS")
if [[ -n "$TARGET" ]]; then COMMON_ARGS+=(--target "$TARGET"); fi
if [[ "$NO_CACHE" == "true" ]]; then COMMON_ARGS+=(--no-cache); fi
if [[ "$PULL" == "true" ]]; then COMMON_ARGS+=(--pull); fi
COMMON_ARGS+=("${BUILD_ARGS[@]}")

need_buildx="false"
if [[ -n "$PLATFORM" || "$PUSH" == "true" || "$LOAD" == "true" ]]; then
  need_buildx="true"
fi

have_buildx="false"
if docker buildx version >/dev/null 2>&1; then
  have_buildx="true"
fi

if [[ "$need_buildx" == "true" && "$have_buildx" == "false" ]]; then
  echo "buildx is required for --platform/--push/--load, but 'docker buildx' is unavailable." >&2
  exit 1
fi

if [[ "$need_buildx" == "true" ]]; then
  # default to --load for single-platform local builds when user didn't specify
  if [[ "$PUSH" == "false" && "$LOAD" == "false" ]]; then
    if [[ -z "$PLATFORM" || "$PLATFORM" != *,* ]]; then
      LOAD="true"
    fi
  fi

  BUILD_ARGS_X=(docker buildx build -t "$FULL_TAG")
  if [[ -n "$PLATFORM" ]]; then BUILD_ARGS_X+=(--platform "$PLATFORM"); fi
  if [[ "$PUSH" == "true" ]]; then BUILD_ARGS_X+=(--push); fi
  if [[ "$LOAD" == "true" ]]; then BUILD_ARGS_X+=(--load); fi
  BUILD_ARGS_X+=("${COMMON_ARGS[@]}")
  BUILD_ARGS_X+=("$CONTEXT")

  echo "+ ${BUILD_ARGS_X[*]}" >&2
  "${BUILD_ARGS_X[@]}"
else
  BUILD_ARGS_D=(docker build -t "$FULL_TAG")
  BUILD_ARGS_D+=("${COMMON_ARGS[@]}")
  BUILD_ARGS_D+=("$CONTEXT")

  echo "+ ${BUILD_ARGS_D[*]}" >&2
  "${BUILD_ARGS_D[@]}"
fi

echo "Built: $FULL_TAG"
