#!/bin/bash
# LOKAHI Installation Script
# Installs lokahi from source or pre-built binary

set -e

VERSION="0.1.0"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
REPO="rickeshtn/lokahi"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 LOKAHI Installer v${VERSION}${NC}"
echo "Installing to: $INSTALL_DIR"
echo ""

# Check prerequisites
check_deps() {
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}❌ Docker not found. Install from https://docker.com${NC}"
        exit 1
    fi

    if ! command -v docker-compose &> /dev/null; then
        echo -e "${RED}❌ Docker Compose not found${NC}"
        exit 1
    fi

    echo -e "${GREEN}✅ Docker and Docker Compose found${NC}"
}

# Download pre-built binary (if available)
install_binary() {
    echo ""
    echo "Downloading pre-built binary..."

    OSTYPE=$(uname -s)
    if [ "$OSTYPE" == "Linux" ]; then
        PLATFORM="linux"
    elif [ "$OSTYPE" == "Darwin" ]; then
        PLATFORM="macos"
    else
        echo -e "${YELLOW}⚠️  Binary not available for $OSTYPE, will build from source${NC}"
        install_from_source
        return
    fi

    DOWNLOAD_URL="https://github.com/$REPO/releases/download/v$VERSION/lokahi-$PLATFORM"

    mkdir -p "$INSTALL_DIR"
    curl -sL "$DOWNLOAD_URL" -o "$INSTALL_DIR/lokahi" || {
        echo -e "${YELLOW}⚠️  Binary download failed, building from source${NC}"
        install_from_source
        return
    }

    chmod +x "$INSTALL_DIR/lokahi"
    echo -e "${GREEN}✅ Binary installed${NC}"
}

# Build from source
install_from_source() {
    echo ""
    echo "Building from source..."

    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}❌ Rust not found. Install from https://rustup.rs${NC}"
        exit 1
    fi

    # Create temp directory
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT

    cd "$TEMP_DIR"

    # Clone repo
    echo "Cloning repository..."
    git clone --depth 1 https://github.com/$REPO.git .

    # Build
    echo "Building binary (this may take a few minutes)..."
    cargo build --release

    # Install
    mkdir -p "$INSTALL_DIR"
    cp target/release/lokahi "$INSTALL_DIR/lokahi"
    chmod +x "$INSTALL_DIR/lokahi"

    echo -e "${GREEN}✅ Built and installed from source${NC}"
}

# Create wrapper script for docker-compose
create_wrapper() {
    mkdir -p "$INSTALL_DIR"

    cat > "$INSTALL_DIR/lokahi-run" << 'EOF'
#!/bin/bash
# LOKAHI Docker wrapper
# Runs lokahi in Docker with current directory mounted

PROJECT_DIR="${1:-.}"

if [ ! -d "$PROJECT_DIR" ]; then
    echo "Error: Directory $PROJECT_DIR not found"
    exit 1
fi

# Download docker-compose.yml if not present
if [ ! -f "$PROJECT_DIR/docker-compose.yml" ]; then
    echo "Downloading docker-compose.yml..."
    mkdir -p "$PROJECT_DIR"
    curl -sL "https://raw.githubusercontent.com/rickeshtn/lokahi/main/docker-compose.yml" \
        -o "$PROJECT_DIR/docker-compose.yml"
fi

cd "$PROJECT_DIR"
docker-compose run --rm lokahi
EOF

    chmod +x "$INSTALL_DIR/lokahi-run"
    echo -e "${GREEN}✅ Docker wrapper script created${NC}"
}

# Setup PATH
setup_path() {
    echo ""
    echo "Setting up PATH..."

    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo ""
        echo -e "${YELLOW}Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):${NC}"
        echo "export PATH=\"\$PATH:$INSTALL_DIR\""
        echo ""
    else
        echo -e "${GREEN}✅ PATH already configured${NC}"
    fi
}

# Main installation flow
main() {
    check_deps

    # Try binary first, fallback to source
    if command -v cargo &> /dev/null; then
        read -p "Build from source? (y/n, default: n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            install_from_source
        else
            install_binary
        fi
    else
        install_binary
    fi

    create_wrapper
    setup_path

    echo ""
    echo -e "${GREEN}✅ LOKAHI installed successfully!${NC}"
    echo ""
    echo "Usage:"
    echo "  lokahi-run [project-dir]   # Run in Docker (recommended)"
    echo "  lokahi                     # Run locally (requires built binary)"
    echo ""
    echo "First run:"
    echo "  cd ~/Projects/my-project"
    echo "  lokahi-run ."
    echo ""
}

main
