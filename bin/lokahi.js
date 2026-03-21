#!/usr/bin/env node

/**
 * LOKAHI CLI Wrapper
 * Downloads and runs the Rust binary
 */

const fs = require('fs');
const path = require('path');
const { execSync, spawn } = require('child_process');
const os = require('os');

const BINARY_NAME = 'lokahi-bin';
const BINARY_DIR = path.join(__dirname, '..', 'bin', 'native');
const VERSION = '0.1.0';

function getOS() {
  const platform = os.platform();
  if (platform === 'linux') return 'linux';
  if (platform === 'darwin') return 'macos';
  throw new Error(`Unsupported platform: ${platform}`);
}

function getBinaryPath() {
  const osType = getOS();
  return path.join(BINARY_DIR, `lokahi-${osType}`);
}

function ensureBinaryExists() {
  const binaryPath = getBinaryPath();

  if (fs.existsSync(binaryPath)) {
    return binaryPath;
  }

  // Try to use docker-compose as fallback
  console.log('🔧 Binary not found, using Docker...');
  console.log('Running: docker-compose run --rm lokahi');
  console.log('');

  try {
    execSync('docker-compose run --rm lokahi', {
      cwd: process.cwd(),
      stdio: 'inherit',
    });
  } catch (err) {
    if (err.status === 130) {
      // User interrupted with Ctrl+C
      process.exit(0);
    }
    throw err;
  }

  process.exit(0);
}

function main() {
  try {
    const binaryPath = ensureBinaryExists();

    // Run the binary with all passed arguments
    const child = spawn(binaryPath, process.argv.slice(2), {
      stdio: 'inherit',
      shell: false,
    });

    child.on('exit', (code) => {
      process.exit(code);
    });

    child.on('error', (err) => {
      console.error('Error running LOKAHI:', err);
      process.exit(1);
    });
  } catch (err) {
    console.error('❌ Error:', err.message);
    console.error('');
    console.error('Installation options:');
    console.error('1. Install with script:  bash <(curl -fsSL https://raw.githubusercontent.com/rickeshtn/lokahi/main/install.sh)');
    console.error('2. Install from source:  cargo install --git https://github.com/rickeshtn/lokahi');
    console.error('3. Run with Docker:      docker-compose run --rm lokahi');
    process.exit(1);
  }
}

main();
