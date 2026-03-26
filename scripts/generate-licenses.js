const { execSync } = require('child_process');
const fs = require('fs');

function getPnpmLicenses(dir, includeDev = false) {
    const mode = includeDev ? 'all' : 'production';
    console.log(`Extracting ${mode} licenses for ${dir}...`);
    try {
        const cmd = `pnpm licenses list --json ${includeDev ? '' : '--prod'} --dir ${dir}`;
        const stdout = execSync(cmd, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], shell: true });
        
        if (!stdout || stdout.trim() === 'No licenses in packages found') return {};

        const data = JSON.parse(stdout);
        const licenses = {};
        for (const [licenseName, packages] of Object.entries(data)) {
            if (!licenses[licenseName]) licenses[licenseName] = new Set();
            for (const pkg of packages) {
                licenses[licenseName].add(`${pkg.name} (${pkg.versions.join(', ')})`);
            }
        }
        return licenses;
    } catch (e) {
        return {};
    }
}

function getCargoLicenses() {
    // Rust usually does not distinguish between runtime and dev-dependencies for licenses 
    // because dev-dependencies are not compiled into the final binary.
    // Therefore, both versions will include all registry dependencies.
    try {
        const stdout = execSync(`cargo metadata --format-version 1`, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], shell: true });
        const data = JSON.parse(stdout);
        const licenses = {};

        for (const pkg of data.packages || []) {
            if (!pkg.source || !pkg.source.startsWith('registry')) continue; // Skip local workspace members
            const licenseName = pkg.license || 'Unknown';
            // Handle composite protocols like "MIT OR Apache-2.0"
            const entries = licenseName.split(/ OR | AND | \/ /).map(l => l.trim().replace(/[()]/g, ''));
            for (const lic of entries) {
                if (!licenses[lic]) licenses[lic] = new Set();
                licenses[lic].add(`${pkg.name} (${pkg.version})`);
            }
        }
        return licenses;
    } catch (e) {
        return {};
    }
}

function generateMarkdown(allLicenses, title) {
    let output = `# ${title}\n\n`;
    output += 'This project uses various open-source components. Below is a list of these components and their respective licenses.\n\n';

    const sortedLicenses = Object.keys(allLicenses).sort();
    for (const lic of sortedLicenses) {
        output += `## ${lic}\n\n`;
        const sortedPkgs = Array.from(allLicenses[lic]).sort();
        for (const pkg of sortedPkgs) {
            output += `- ${pkg}\n`;
        }
        output += '\n';
    }
    return output;
}

function main() {
    const cargo = getCargoLicenses();
    const lucide = { 'ISC': new Set(['Lucide (ISC License)']) };

    // 1. Generate Production version (Prod only)
    const prodLicenses = {};
    const merge = (target, source) => {
        for (const [lic, pkgs] of Object.entries(source)) {
            if (!target[lic]) target[lic] = new Set();
            for (const p of pkgs) target[lic].add(p);
        }
    };

    merge(prodLicenses, getPnpmLicenses('docs', false));
    merge(prodLicenses, getPnpmLicenses('frontend', false));
    merge(prodLicenses, cargo);
    merge(prodLicenses, lucide);
    fs.writeFileSync('THIRD-PARTY-LICENSE.md', generateMarkdown(prodLicenses, 'Third-Party Licenses (Production)'));
    console.log('Created THIRD-PARTY-LICENSE.md');

    // 2. Generate Full version (Including Dev)
    const fullLicenses = {};
    merge(fullLicenses, getPnpmLicenses('docs', true));
    merge(fullLicenses, getPnpmLicenses('frontend', true));
    merge(fullLicenses, cargo);
    merge(fullLicenses, lucide);
    fs.writeFileSync('THIRD-PARTY-LICENSE-FULL.md', generateMarkdown(fullLicenses, 'Third-Party Licenses (All Dependencies)'));
    console.log('Created THIRD-PARTY-LICENSE-FULL.md');
}

main();
