const path = require('node:path');
const fs = require('node:fs');

const version = process.env.GINSENG_VERSION?.replace('v', '');
if (!version) {
  throw new Error('GINSENG_VERSION environment variable not set');
}

// Transform beta version to MSI-compatible format
// e.g., "0.1.0-beta.1" -> "0.1.0-1"
const msiCompatibleVersion = version.replace(/-beta\.(\d+)/, '-$1');

const tauriConfigPath = path.join(__dirname, '../src-tauri/tauri.conf.json');
const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, 'utf8'));

tauriConfig.version = msiCompatibleVersion;

console.log(`Writing MSI-compatible version ${msiCompatibleVersion} to ${tauriConfigPath}`);
fs.writeFileSync(tauriConfigPath, JSON.stringify(tauriConfig, null, 2));