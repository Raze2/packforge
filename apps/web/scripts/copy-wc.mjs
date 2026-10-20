// Copies the built <packforge-designer> bundle into the Laravel package so
// `php artisan vendor:publish --tag=packforge-assets` can publish it.
import { copyFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const src = fileURLToPath(new URL('../dist/wc/packforge-designer.js', import.meta.url));
const dir = fileURLToPath(new URL('../../../integrations/laravel/resources/dist/', import.meta.url));
mkdirSync(dir, { recursive: true });
copyFileSync(src, `${dir}packforge-designer.js`);
console.log(`copied web component to ${dir}`);
