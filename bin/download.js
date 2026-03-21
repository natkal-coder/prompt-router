#!/usr/bin/env node

/**
 * Post-install script for npm
 * Downloads pre-built binary or suggests alternatives
 */

const fs = require('fs');
const path = require('path');
const os = require('os');
const https = require('https');

const BINARY_DIR = path.join(__dirname, 'native');
const REPO = 'rickeshtn/lokahi';
const VERSION = '0.1.0';

function getOS() {
  const platform = os.platform();
  if (platform === 'linux') return 'linux';
  if (platform === 'darwin') return 'macos';
  return null;
}

function downloadFile(url, dest) {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);
    https
      .get(url, (response) => {
        if (response.statusCode === 404) {
          file.close();
          fs.unlinkSync(dest);
          resolve(false);
          return;
        }

        response.pipe(file);
        file.on('finish', () => {
          file.close();
          resolve(true);
        });
      })
      .on('error', (err) => {
        file.close();
        fs.unlinkSync(dest);
        reject(err);
      });
  });
}

async function main() {
  const osType = getOS();

  if (!osType) {
    console.log('⚠️  Platform not supported for binary download');
    console.log('Install from source instead:');
    console.log('  cargo install --git https://github.com/rickeshtn/lokahi');
    return;
  }

  // Create binary directory
  if (!fs.existsSync(BINARY_DIR)) {
    fs.mkdirSync(BINARY_DIR, { recursive: true });
  }

  const binaryPath = path.join(BINARY_DIR, `lokahi-${osType}`);

  if (fs.existsSync(binaryPath)) {
    fs.chmodSync(binaryPath, 0o755);
    console.log('✅ LOKAHI binary ready');
    return;
  }

  console.log('📥 Downloading LOKAHI binary...');

  const downloadUrl = `https://github.com/${REPO}/releases/download/v${VERSION}/lokahi-${osType}`;

  try {
    const success = await downloadFile(downloadUrl, binaryPath);

    if (success) {
      fs.chmodSync(binaryPath, 0o755);
      console.log('✅ LOKAHI binary downloaded');
    } else {
      console.log('');
      console.log('⚠️  Pre-built binary not available yet.');
      console.log('Using Docker instead (docker-compose required)');
      console.log('');
      console.log('To build from source:');
      console.log('  cargo install --git https://github.com/rickeshtn/lokahi');
    }
  } catch (err) {
    console.error('⚠️  Download failed:', err.message);
    console.log('');
    console.log('Fallback: Using Docker wrapper');
    console.log('Run with: lokahi (requires docker-compose)');
  }
}

main().catch(console.error);
