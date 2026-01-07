#!/bin/bash
set -e

# Deploy Script
# Unified script for Docker Hub deployment
# Supports both direct push and GitHub Actions (via git tag)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

log_header() {
    echo -e "${CYAN}$1${NC}"
}

# ============================================
# Common Functions
# ============================================

# Get version from Cargo.toml
get_version() {
    grep '^version' "$PROJECT_ROOT/Cargo.toml" | head -1 | sed 's/.*= *"\(.*\)"/\1/'
}

# Get latest git tag version
get_latest_tag() {
    git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//' || echo "0.0.0"
}

# Parse version into components
parse_version() {
    local version="$1"
    echo "$version" | tr '.' ' '
}

# Bump version based on type (major, minor, patch)
bump_version() {
    local current="$1"
    local bump_type="$2"
    
    local major minor patch
    read major minor patch <<< $(parse_version "$current")
    
    case "$bump_type" in
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        patch)
            patch=$((patch + 1))
            ;;
    esac
    
    echo "${major}.${minor}.${patch}"
}

# Update version in Cargo.toml
update_cargo_version() {
    local new_version="$1"
    sed -i '' "s/^version = \".*\"/version = \"$new_version\"/" "$PROJECT_ROOT/Cargo.toml"
}

# Interactive version selection
select_version_bump() {
    local current_version="$1"
    
    local major minor patch
    read major minor patch <<< $(parse_version "$current_version")
    
    local new_major="$((major + 1)).0.0"
    local new_minor="${major}.$((minor + 1)).0"
    local new_patch="${major}.${minor}.$((patch + 1))"
    
    # Display menu to stderr so it doesn't interfere with return value
    echo "" >&2
    echo -e "${CYAN}============================================${NC}" >&2
    echo -e "${CYAN}  Version Selection${NC}" >&2
    echo -e "${CYAN}============================================${NC}" >&2
    echo "" >&2
    echo -e "${GREEN}[INFO]${NC} Current version: ${CYAN}v${current_version}${NC}" >&2
    echo "" >&2
    echo "Select version bump type:" >&2
    echo "" >&2
    echo -e "  [1] Major  : v${current_version} → ${GREEN}v${new_major}${NC}  (Breaking changes)" >&2
    echo -e "  [2] Minor  : v${current_version} → ${GREEN}v${new_minor}${NC}  (New features)" >&2
    echo -e "  [3] Patch  : v${current_version} → ${GREEN}v${new_patch}${NC}  (Bug fixes)" >&2
    echo -e "  [4] Custom : Enter custom version" >&2
    echo -e "  [0] Cancel" >&2
    echo "" >&2
    
    local choice
    read -p "Enter choice [1-4, 0 to cancel]: " choice
    
    case "$choice" in
        1)
            echo "$new_major"
            ;;
        2)
            echo "$new_minor"
            ;;
        3)
            echo "$new_patch"
            ;;
        4)
            local custom_version
            read -p "Enter custom version (e.g., 1.2.3): " custom_version
            # Validate format
            if [[ "$custom_version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
                echo "$custom_version"
            else
                echo -e "${RED}[ERROR]${NC} Invalid version format. Use X.Y.Z format." >&2
                exit 1
            fi
            ;;
        0)
            echo -e "${GREEN}[INFO]${NC} Cancelled." >&2
            exit 0
            ;;
        *)
            echo -e "${RED}[ERROR]${NC} Invalid choice." >&2
            exit 1
            ;;
    esac
}

# ============================================
# Direct Docker Push Functions
# ============================================

# Load environment variables from .env file
load_env() {
    local env_file="$PROJECT_ROOT/.env"
    
    if [[ -f "$env_file" ]]; then
        log_info "Loading environment from .env file..."
        set -a
        source "$env_file"
        set +a
    else
        log_error ".env file not found at $env_file"
        log_info "Please copy .env.example to .env and fill in your Docker Hub credentials"
        exit 1
    fi
}

# Validate required environment variables
validate_env() {
    local missing=()
    
    # Support both naming conventions
    DOCKER_USERNAME="${DOCKER_USERNAME:-$DOCKER_HUB_USERNAME}"
    DOCKER_ACCESS_TOKEN="${DOCKER_ACCESS_TOKEN:-$DOCKER_HUB_TOKEN}"
    IMAGE_NAME="${IMAGE_NAME:-gen-serving-gateway}"
    
    [[ -z "$DOCKER_USERNAME" ]] && missing+=("DOCKER_USERNAME or DOCKER_HUB_USERNAME")
    [[ -z "$DOCKER_ACCESS_TOKEN" ]] && missing+=("DOCKER_ACCESS_TOKEN or DOCKER_HUB_TOKEN")
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing required environment variables: ${missing[*]}"
        exit 1
    fi
}

# Login to Docker Hub
docker_login() {
    log_step "Logging in to Docker Hub..."
    echo "$DOCKER_ACCESS_TOKEN" | docker login -u "$DOCKER_USERNAME" --password-stdin
    log_info "Login successful"
}

# Build Docker image
build_image() {
    local full_image_name="$DOCKER_USERNAME/$IMAGE_NAME"
    local version="$1"
    
    log_step "Building Docker image: $full_image_name:$version"
    
    docker build \
        --platform linux/amd64 \
        -t "$full_image_name:$version" \
        -t "$full_image_name:latest" \
        -f "$PROJECT_ROOT/Dockerfile" \
        "$PROJECT_ROOT"
    
    log_info "Image built successfully"
}

# Push Docker image
push_image() {
    local full_image_name="$DOCKER_USERNAME/$IMAGE_NAME"
    local version="$1"
    
    log_step "Pushing $full_image_name:$version to Docker Hub..."
    docker push "$full_image_name:$version"
    
    log_step "Pushing $full_image_name:latest to Docker Hub..."
    docker push "$full_image_name:latest"
    
    log_info "Push completed successfully!"
    echo ""
    log_info "Image available at: https://hub.docker.com/r/$DOCKER_USERNAME/$IMAGE_NAME"
}

# Cleanup Docker login
cleanup_docker() {
    log_info "Logging out from Docker Hub..."
    docker logout 2>/dev/null || true
}

# Direct push mode
direct_push() {
    local version="$1"
    local build_only="$2"
    local push_only="$3"
    
    load_env
    validate_env
    
    if [[ -z "$version" ]]; then
        version=$(get_version)
    fi
    
    echo ""
    log_header "============================================"
    log_header "  Direct Docker Push"
    log_header "============================================"
    echo ""
    log_info "Image: $DOCKER_USERNAME/$IMAGE_NAME"
    log_info "Version: $version"
    echo ""
    
    trap cleanup_docker EXIT
    
    docker_login
    
    if [[ "$push_only" != true ]]; then
        build_image "$version"
    fi
    
    if [[ "$build_only" != true ]]; then
        push_image "$version"
    fi
    
    echo ""
    log_info "Done!"
}

# ============================================
# GitHub Actions Release Functions
# ============================================

# Check if tag exists
tag_exists() {
    local tag="$1"
    git tag -l "$tag" | grep -q "$tag"
}

# Check for uncommitted changes and auto-commit if needed
check_and_commit_changes() {
    local version="$1"
    local dry_run="$2"
    
    # Check for any changes (staged or unstaged)
    if git diff --quiet HEAD 2>/dev/null && git diff --cached --quiet 2>/dev/null; then
        log_info "Working directory is clean"
        return 0
    fi
    
    log_warn "Uncommitted changes detected. Auto-committing..."
    
    # Generate commit message with timestamp
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    local commit_msg="release: v${version} @ ${timestamp}"
    
    if [[ "$dry_run" == true ]]; then
        echo "  [DRY-RUN] git add -A"
        echo "  [DRY-RUN] git commit -m \"$commit_msg\""
        echo "  [DRY-RUN] git push origin $(git branch --show-current)"
    else
        # Stage all changes
        log_step "Staging all changes..."
        git add -A
        log_info "Changes staged"
        
        # Commit
        log_step "Committing changes..."
        git commit -m "$commit_msg"
        log_info "Changes committed"
        
        # Push to current branch
        local current_branch=$(git branch --show-current)
        log_step "Pushing to origin/$current_branch..."
        git push origin "$current_branch"
        log_info "Changes pushed to origin/$current_branch"
    fi
    
    echo ""
}

# GitHub Actions release mode
github_release() {
    local version="$1"
    local message="$2"
    local force="$3"
    local dry_run="$4"
    local interactive="$5"
    
    cd "$PROJECT_ROOT"
    
    # Get current/latest version
    local current_version=$(get_latest_tag)
    local cargo_version=$(get_version)
    
    # Interactive version selection
    if [[ "$interactive" == true ]] || [[ -z "$version" ]]; then
        version=$(select_version_bump "$current_version")
        
        # Update Cargo.toml if version changed
        if [[ "$version" != "$cargo_version" ]]; then
            log_step "Updating Cargo.toml version to $version..."
            if [[ "$dry_run" != true ]]; then
                update_cargo_version "$version"
                log_info "Cargo.toml updated"
            else
                echo "  [DRY-RUN] Update Cargo.toml version to $version"
            fi
        fi
    fi
    
    local tag="v$version"
    
    if [[ -z "$message" ]]; then
        message="Release $tag"
    fi
    
    echo ""
    log_header "============================================"
    log_header "  GitHub Actions Release"
    log_header "============================================"
    echo ""
    log_info "Previous version: v$current_version"
    log_info "New version: $tag"
    log_info "Message: $message"
    echo ""
    
    # Check working tree and auto-commit if needed
    log_step "Checking working directory..."
    check_and_commit_changes "$version" "$dry_run"
    
    # Check if tag exists
    log_step "Checking if tag exists..."
    if tag_exists "$tag"; then
        if [[ "$force" == true ]]; then
            log_warn "Tag $tag already exists. Will be overwritten (--force)"
            if [[ "$dry_run" != true ]]; then
                git tag -d "$tag" 2>/dev/null || true
                git push origin ":refs/tags/$tag" 2>/dev/null || true
            fi
        else
            log_error "Tag $tag already exists."
            log_info "Use --force to overwrite, or update version in Cargo.toml"
            exit 1
        fi
    else
        log_info "Tag $tag is available"
    fi
    
    # Create tag
    log_step "Creating tag $tag..."
    if [[ "$dry_run" == true ]]; then
        echo "  [DRY-RUN] git tag -a $tag -m \"$message\""
    else
        git tag -a "$tag" -m "$message"
        log_info "Tag created locally"
    fi
    
    # Push tag to GitHub
    log_step "Pushing tag to GitHub..."
    if [[ "$dry_run" == true ]]; then
        echo "  [DRY-RUN] git push origin $tag"
    else
        git push origin "$tag"
        log_info "Tag pushed to GitHub"
    fi
    
    echo ""
    log_header "============================================"
    if [[ "$dry_run" == true ]]; then
        log_warn "DRY RUN - No changes were made"
    else
        log_info "Release $tag created successfully!"
        echo ""
        log_info "GitHub Actions will now:"
        echo "  1. Build Docker image"
        echo "  2. Push to Docker Hub as:"
        echo "     - \$DOCKER_USERNAME/gen-serving-gateway:$version"
        echo "     - \$DOCKER_USERNAME/gen-serving-gateway:latest"
        echo ""
        log_info "Monitor progress at:"
        local repo_url=$(git remote get-url origin 2>/dev/null | sed 's/.*github.com[:/]\(.*\)\.git/\1/' || echo "your-repo")
        echo "  https://github.com/$repo_url/actions"
    fi
    log_header "============================================"
    echo ""
}

# ============================================
# Main
# ============================================

usage() {
    echo "Usage: $0 <MODE> [OPTIONS]"
    echo ""
    echo "Unified deployment script for Docker Hub"
    echo ""
    echo "Modes:"
    echo "  direct          Build and push Docker image directly"
    echo "  release         Create git tag and trigger GitHub Actions"
    echo ""
    echo "Direct Mode Options:"
    echo "  -v, --version VERSION   Override version tag (default: from Cargo.toml)"
    echo "  -b, --build-only        Build image without pushing"
    echo "  -p, --push-only         Push existing image without building"
    echo ""
    echo "Release Mode Options:"
    echo "  -v, --version VERSION   Set specific version (skips interactive selection)"
    echo "  -m, --message MESSAGE   Tag message (default: 'Release vX.X.X')"
    echo "  -f, --force             Force create tag even if it exists"
    echo "  -d, --dry-run           Show what would be done without executing"
    echo ""
    echo "Common Options:"
    echo "  -h, --help              Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 direct                    # Direct build and push"
    echo "  $0 direct -v 1.0.0           # Direct push with specific version"
    echo "  $0 direct -b                 # Build only (no push)"
    echo "  $0 release                   # Interactive version selection + release"
    echo "  $0 release -v 1.0.0          # Release specific version (no prompt)"
    echo "  $0 release -d                # Dry run release"
    echo ""
    echo "Version Selection (release mode):"
    echo "  When run without -v, prompts to select:"
    echo "    [1] Major  : x.0.0 (Breaking changes)"
    echo "    [2] Minor  : 0.x.0 (New features)"
    echo "    [3] Patch  : 0.0.x (Bug fixes)"
    echo "    [4] Custom : Enter custom version"
}

main() {
    if [[ $# -lt 1 ]]; then
        usage
        exit 1
    fi
    
    local mode="$1"
    shift
    
    case "$mode" in
        direct)
            local version=""
            local build_only=false
            local push_only=false
            
            while [[ $# -gt 0 ]]; do
                case $1 in
                    -v|--version) version="$2"; shift 2 ;;
                    -b|--build-only) build_only=true; shift ;;
                    -p|--push-only) push_only=true; shift ;;
                    -h|--help) usage; exit 0 ;;
                    *) log_error "Unknown option: $1"; usage; exit 1 ;;
                esac
            done
            
            direct_push "$version" "$build_only" "$push_only"
            ;;
        
        release)
            local version=""
            local message=""
            local force=false
            local dry_run=false
            local interactive=true  # Default to interactive mode
            
            while [[ $# -gt 0 ]]; do
                case $1 in
                    -v|--version) version="$2"; interactive=false; shift 2 ;;
                    -m|--message) message="$2"; shift 2 ;;
                    -f|--force) force=true; shift ;;
                    -d|--dry-run) dry_run=true; shift ;;
                    -h|--help) usage; exit 0 ;;
                    *) log_error "Unknown option: $1"; usage; exit 1 ;;
                esac
            done
            
            github_release "$version" "$message" "$force" "$dry_run" "$interactive"
            ;;
        
        -h|--help)
            usage
            exit 0
            ;;
        
        *)
            log_error "Unknown mode: $mode"
            echo ""
            usage
            exit 1
            ;;
    esac
}

main "$@"

