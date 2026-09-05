// Native icon assets derived from the single vector source in assets/logo.svg.
// Setup and the visual contract live in docs/VISUALS.md.
import { readFile, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const require = createRequire(join(root, '.agent/logo-tools/package.json'));
const { Resvg } = require('@resvg/resvg-js');
const packageInfo = JSON.parse(await readFile(
  join(root, '.agent/logo-tools/node_modules/@resvg/resvg-js/package.json'), 'utf8',
));
if (packageInfo.version !== '2.6.2') {
  throw new Error('Icon rasterizer version must be 2.6.2');
}
if (process.argv.slice(2).some((argument) => argument !== '--check')) {
  throw new Error('Usage: node scripts/render-logo.mjs [--check]');
}
const check = process.argv.includes('--check');
const source = await readFile(join(root, 'assets/logo.svg'));
const sizes = [16, 20, 24, 32, 48, 64, 128, 256];
const images = sizes.map((size) => new Resvg(source, {
  fitTo: { mode: 'width', value: size },
  font: { loadSystemFonts: false },
}).render().asPng());

// ICO admits a separate PNG at each requested size. Each one is rasterized
// from the vector, so the smallest icons do not inherit a downscaled blur.
const directory = Buffer.alloc(6 + 16 * images.length);
directory.writeUInt16LE(1, 2);
directory.writeUInt16LE(images.length, 4);
let offset = directory.length;
for (const [index, image] of images.entries()) {
  const entry = 6 + index * 16;
  directory[entry] = sizes[index] === 256 ? 0 : sizes[index];
  directory[entry + 1] = directory[entry];
  directory.writeUInt16LE(1, entry + 4);
  directory.writeUInt16LE(32, entry + 6);
  directory.writeUInt32LE(image.length, entry + 8);
  directory.writeUInt32LE(offset, entry + 12);
  offset += image.length;
}
const png = images.at(-1);
const icnsHeader = Buffer.alloc(16);
icnsHeader.write('icns', 0, 'ascii');
icnsHeader.writeUInt32BE(16 + png.length, 4);
icnsHeader.write('ic08', 8, 'ascii');
icnsHeader.writeUInt32BE(8 + png.length, 12);
for (const [name, bytes] of [
  ['logo.png', png],
  ['logo.ico', Buffer.concat([directory, ...images])],
  ['logo.icns', Buffer.concat([icnsHeader, png])],
]) {
  const path = join(root, 'assets', name);
  if (check) {
    const current = await readFile(path);
    if (!current.equals(bytes)) {
      throw new Error(`${name} does not match assets/logo.svg`);
    }
  } else {
    await writeFile(path, bytes);
  }
  process.stdout.write(`${check ? 'verified' : 'wrote'} assets/${name}\n`);
}
