// A small static server for the built site that behaves like GitHub Pages where it matters for
// measuring and testing: gzip at about the level Pages uses (5), `application/wasm` for the
// engine, `cache-control: max-age=600`, a 404 for missing files (so a site built without data
// behaves as it would on Pages) and index.html for navigations. It counts every byte it sends,
// per path, so a test can say exactly what a first visit transferred.
//
//   import { serve } from './serve.mjs';
//   const site = await serve({ root: 'dist', base: '/' });
//   ... site.url, site.sent(), site.reset(), site.close()
import { createServer } from 'node:http';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { extname, join, normalize } from 'node:path';
import { gzipSync } from 'node:zlib';

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.json': 'application/json; charset=utf-8',
  '.webmanifest': 'application/manifest+json',
  '.map': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.csv': 'text/csv; charset=utf-8',
  '.toml': 'application/toml',
};
const COMPRESSIBLE = new Set(['.html', '.js', '.mjs', '.css', '.svg', '.json', '.webmanifest', '.map', '.wasm', '.csv', '.toml']);

/**
 * @param {{ root: string, base?: string, gzip?: boolean, port?: number, delayMs?: number }} options
 */
export async function serve({ root, base = '/', gzip = true, port = 0, delayMs = 0 }) {
  const cache = new Map();
  /** path -> { requests, bytes } */
  let sent = new Map();
  let offline = false;

  const body = (file, compress) => {
    const key = `${file}|${compress}`;
    const stat = statSync(file);
    const hit = cache.get(key);
    if (hit && hit.mtime === stat.mtimeMs) return hit.bytes;
    const raw = readFileSync(file);
    const bytes = compress ? gzipSync(raw, { level: 5 }) : raw;
    cache.set(key, { mtime: stat.mtimeMs, bytes });
    return bytes;
  };

  const server = createServer((req, res) => {
    const url = new URL(req.url, 'http://x');
    const path = decodeURIComponent(url.pathname);
    if (offline) {
      req.socket.destroy();
      return;
    }
    if (!path.startsWith(base)) {
      res.writeHead(404).end();
      return;
    }
    const rel = normalize(path.slice(base.length) || 'index.html').replace(/^(\.\.[/\\])+/, '');
    let file = join(root, rel);
    const isFile = existsSync(file) && statSync(file).isFile();
    if (!isFile) {
      // Pages answers a missing file with 404; a navigation (no extension) gets the app.
      if (extname(rel)) {
        res.writeHead(404, { 'content-type': 'text/html; charset=utf-8' }).end('<!doctype html><title>404</title>');
        return;
      }
      file = join(root, 'index.html');
    }
    const ext = extname(file);
    const accepts = /\bgzip\b/.test(String(req.headers['accept-encoding'] ?? ''));
    const compress = gzip && accepts && COMPRESSIBLE.has(ext);
    const bytes = body(file, compress);
    const headers = {
      'content-type': TYPES[ext] ?? 'application/octet-stream',
      'content-length': String(bytes.length),
      'cache-control': 'max-age=600',
    };
    if (compress) headers['content-encoding'] = 'gzip';
    const record = sent.get(rel) ?? { requests: 0, bytes: 0 };
    record.requests += 1;
    record.bytes += bytes.length;
    sent.set(rel, record);
    const reply = () => {
      res.writeHead(200, headers);
      res.end(req.method === 'HEAD' ? undefined : bytes);
    };
    if (delayMs > 0) setTimeout(reply, delayMs);
    else reply();
  });
  await new Promise((resolve) => server.listen(port, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  return {
    origin,
    url: `${origin}${base}`,
    /** Bytes sent since the last reset, per path, and in total. */
    sent() {
      const files = [...sent.entries()].map(([path, r]) => ({ path, ...r })).sort((a, b) => b.bytes - a.bytes);
      return { files, total: files.reduce((s, f) => s + f.bytes, 0) };
    },
    reset() {
      sent = new Map();
    },
    /** Drop every connection, as if the network were gone. */
    setOffline(value) {
      offline = value;
    },
    close() {
      return new Promise((resolve) => server.close(resolve));
    },
  };
}
