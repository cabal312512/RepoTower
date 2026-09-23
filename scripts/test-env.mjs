// Load before Playwright, which captures its artifact directory during import.
import fs from 'node:fs';
import path from 'node:path';

const temp = path.resolve(import.meta.dirname, '..', '.tmp');
fs.mkdirSync(temp, { recursive: true });
process.env.TEMP = temp;
process.env.TMP = temp;
process.env.TMPDIR = temp;
