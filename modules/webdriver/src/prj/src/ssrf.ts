import * as dns from 'node:dns/promises';
import * as net from 'node:net';
import type * as pup from 'puppeteer-core';
import * as logger from './logging.js';

/**
 * SSRF guard for the webdriver.
 *
 * Chrome resolves DNS and follows redirects itself, so it never passes through
 * any host-side filter. Without this guard a contract could point an otherwise
 * valid public URL (or one of the page's sub-resources) at an internal address
 * such as cloud metadata (169.254.169.254), loopback or an RFC1918 host.
 *
 * We intercept every request the page makes (top-level navigation,
 * sub-resources and each redirect hop), resolve the host ourselves and abort
 * the request when any resolved address is not globally routable.
 *
 * NOTE: resolution happens here while Chrome connects separately, so a low-TTL
 * attacker controlling DNS could in theory return a routable address to us and
 * a private one to Chrome. We mitigate by failing closed: if *any* address a
 * host resolves to is private, the host is blocked outright.
 */

function parseIPv4(ip: string): [number, number, number, number] | null {
	const parts = ip.split('.');
	if (parts.length !== 4) {
		return null;
	}
	const nums: number[] = [];
	for (const part of parts) {
		if (!/^\d{1,3}$/.test(part)) {
			return null;
		}
		const n = Number(part);
		if (n > 255) {
			return null;
		}
		nums.push(n);
	}
	return [nums[0]!, nums[1]!, nums[2]!, nums[3]!];
}

/** Whether an IPv4 address is not safe to connect to (not globally routable). */
function ipv4IsBad(ip: string): boolean {
	const octets = parseIPv4(ip);
	if (octets === null) {
		return true; // unparseable => fail closed
	}
	const [a, b, c] = octets;
	return (
		a === 0 || // 0.0.0.0/8 "this" network
		a === 10 || // 10.0.0.0/8 private
		a === 127 || // 127.0.0.0/8 loopback
		(a === 169 && b === 254) || // 169.254.0.0/16 link-local
		(a === 172 && b >= 16 && b <= 31) || // 172.16.0.0/12 private
		(a === 192 && b === 168) || // 192.168.0.0/16 private
		(a === 100 && b >= 64 && b <= 127) || // 100.64.0.0/10 CGNAT
		(a === 192 && b === 0 && c === 2) || // 192.0.2.0/24 documentation
		(a === 198 && b >= 18 && b <= 19) || // 198.18.0.0/15 benchmarking
		(a === 198 && b === 51 && c === 100) || // 198.51.100.0/24 documentation
		(a === 203 && b === 0 && c === 113) || // 203.0.113.0/24 documentation
		a >= 224 // 224.0.0.0/3 multicast + reserved + broadcast
	);
}

function parseIPv6(ip: string): number[] | null {
	const withoutZone = ip.split('%')[0] ?? '';
	const halves = withoutZone.split('::');
	if (halves.length > 2) {
		return null;
	}

	const parseGroups = (segment: string): number[] | null => {
		if (segment === '') {
			return [];
		}
		const parts = segment.split(':');
		const groups: number[] = [];
		for (let i = 0; i < parts.length; i++) {
			const part = parts[i]!;
			if (part.includes('.')) {
				// embedded IPv4 is only valid as the final element
				if (i !== parts.length - 1) {
					return null;
				}
				const octets = parseIPv4(part);
				if (octets === null) {
					return null;
				}
				const [a, b, c, d] = octets;
				groups.push((a << 8) | b);
				groups.push((c << 8) | d);
			} else {
				if (!/^[0-9a-fA-F]{1,4}$/.test(part)) {
					return null;
				}
				groups.push(parseInt(part, 16));
			}
		}
		return groups;
	};

	if (halves.length === 1) {
		const groups = parseGroups(halves[0]!);
		if (groups === null || groups.length !== 8) {
			return null;
		}
		return groups;
	}

	const head = parseGroups(halves[0]!);
	const tail = parseGroups(halves[1]!);
	if (head === null || tail === null) {
		return null;
	}
	const missing = 8 - head.length - tail.length;
	if (missing < 1) {
		return null; // "::" must stand for at least one zero group
	}
	return [...head, ...new Array<number>(missing).fill(0), ...tail];
}

/** Whether an IPv6 address is not safe to connect to (not globally routable). */
function ipv6IsBad(ip: string): boolean {
	const groups = parseIPv6(ip);
	if (groups === null || groups.length !== 8) {
		return true; // fail closed
	}

	// an IPv4-mapped address (::ffff:a.b.c.d) is only as safe as its IPv4 part
	if (
		groups[0] === 0 &&
		groups[1] === 0 &&
		groups[2] === 0 &&
		groups[3] === 0 &&
		groups[4] === 0 &&
		groups[5] === 0xffff
	) {
		const g6 = groups[6]!;
		const g7 = groups[7]!;
		const v4 = `${g6 >> 8}.${g6 & 0xff}.${g7 >> 8}.${g7 & 0xff}`;
		return ipv4IsBad(v4);
	}

	if (groups.every((g) => g === 0)) {
		return true; // :: unspecified
	}
	if (groups.slice(0, 7).every((g) => g === 0) && groups[7] === 1) {
		return true; // ::1 loopback
	}

	const first = groups[0]!;
	return (
		(first & 0xfe00) === 0xfc00 || // fc00::/7 unique-local
		(first & 0xffc0) === 0xfe80 || // fe80::/10 link-local
		(first & 0xffc0) === 0xfec0 || // fec0::/10 deprecated site-local
		(first & 0xff00) === 0xff00 // ff00::/8 multicast
	);
}

/** Whether a resolved address must not be connected to (SSRF guard). */
export function ipIsBad(ip: string): boolean {
	switch (net.isIP(ip)) {
		case 4:
			return ipv4IsBad(ip);
		case 6:
			return ipv6IsBad(ip);
		default:
			return true; // not a bare IP => fail closed
	}
}

/** Resolve a URL host to its addresses; IP literals resolve to themselves. */
async function resolveHost(host: string): Promise<string[]> {
	const stripped =
		host.startsWith('[') && host.endsWith(']') ? host.slice(1, -1) : host;
	if (net.isIP(stripped) !== 0) {
		return [stripped];
	}
	const results = await dns.lookup(stripped, { all: true });
	return results.map((r) => r.address);
}

// Schemes that perform no network access and are always allowed.
const NETWORKLESS_SCHEMES = new Set(['data:', 'blob:', 'about:']);
// Schemes that go to a host and must be checked.
const NETWORK_SCHEMES = new Set(['http:', 'https:', 'ws:', 'wss:']);

async function isUrlAllowed(rawUrl: string): Promise<boolean> {
	let parsed: URL;
	try {
		parsed = new URL(rawUrl);
	} catch {
		return false;
	}
	if (NETWORKLESS_SCHEMES.has(parsed.protocol)) {
		return true;
	}
	if (!NETWORK_SCHEMES.has(parsed.protocol)) {
		return false; // file:, ftp:, chrome:, ...
	}
	let addrs: string[];
	try {
		addrs = await resolveHost(parsed.hostname);
	} catch {
		return false; // resolution failure => block
	}
	if (addrs.length === 0) {
		return false;
	}
	// fail closed: block if *any* resolved address is non-globally-routable
	return !addrs.some(ipIsBad);
}

/**
 * Enable request interception on `page` and abort every request whose target
 * resolves to a non-globally-routable address.
 */
export async function installSsrfGuard(page: pup.Page): Promise<void> {
	await page.setRequestInterception(true);
	page.on('request', (request) => {
		void (async () => {
			const url = request.url();
			let allowed: boolean;
			try {
				allowed = await isUrlAllowed(url);
			} catch {
				allowed = false;
			}
			try {
				if (allowed) {
					await request.continue();
				} else {
					logger.log('warn', 'ssrf guard blocked request', { url });
					await request.abort('blockedbyclient');
				}
			} catch (e) {
				// the request may already be resolved or the page may be gone
				logger.log('debug', 'ssrf guard could not finalize request', {
					url,
					error: (e as Error).message,
				});
			}
		})();
	});
}
