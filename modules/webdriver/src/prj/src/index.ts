import puppeteer, * as pup from 'puppeteer-core';
import http from 'http';
import { Command } from 'commander';

import * as logger from './logging.js';
import * as chromeBrowser from './browser/chrome.js';
import * as ssrf from './ssrf.js';
import { envDurationMs, envInt, formatDurationMs } from './duration.js';

interface NavigationOptions {
	waitUntil?: pup.PuppeteerLifeCycleEvent;
	timeout?: number;
}

interface RenderOptions {
	loadTimeout?: number;
	waitAfterLoaded?: number;
	waitUntil?: pup.PuppeteerLifeCycleEvent;
	maxPageHeapMB?: number;
}

const program = new Command();
program
	.name('puppeteer-webdriver')
	.description('Puppeteer-based web scraping server')
	.version('1.0.0')
	.option('-p, --port <number>', 'port to run the server on', '4444')
	.parse();

const options = program.opts();

const STATUS_I_AM_A_TEAPOT = 418;

const DEFAULT_MAX_PAGE_HEAP_MB = envInt('GVM_WEBDRIVER_MAX_PAGE_HEAP_MB', 1024);

const MAX_WAIT_AFTER_LOADED_MS = envDurationMs(
	'GVM_WEBDRIVER_MAX_WAIT_AFTER_LOADED',
	'60s',
);

const HEALTHCHECK_CACHE_DURATION_MS = envDurationMs(
	'GVM_WEBDRIVER_HEALTHCHECK_CACHE_DURATION',
	'5m',
);
let lastSuccessfulRenderTime: number = 0;

function updateLastSuccessfulRenderTime() {
	lastSuccessfulRenderTime = Date.now();
}

function normalizeWhitespace(contents: string): string {
	return contents
		.split('\n')
		.map((line) => line.trim().replace(/\s+/g, ' '))
		.join('\n')
		.replace(/\n{2,}/g, '\n\n');
}

function getNavigationErrorStatus(error: any): number {
	if (error.name === 'TimeoutError') {
		return 408; // Request Timeout
	} else if (error.message?.includes('net::ERR_NAME_NOT_RESOLVED')) {
		return 502; // Bad Gateway
	} else if (error.message?.includes('net::ERR_CONNECTION_REFUSED')) {
		return 503; // Service Unavailable
	} else if (error.message?.includes('net::ERR_CERT_')) {
		return 495; // SSL Certificate Error
	} else if (error.message?.includes('net::ERR_INTERNET_DISCONNECTED')) {
		return 503; // Service Unavailable
	} else if (error.message?.includes('net::ERR_BLOCKED_BY_CLIENT')) {
		return 403; // Forbidden (SSRF guard)
	}
	return STATUS_I_AM_A_TEAPOT; // Unknown error
}

function getNavigationErrorMessage(error: any): string {
	if (error.name === 'TimeoutError') {
		return 'Navigation timeout';
	} else if (error.message?.includes('net::ERR_NAME_NOT_RESOLVED')) {
		return 'DNS resolution failed';
	} else if (error.message?.includes('net::ERR_CONNECTION_REFUSED')) {
		return 'Connection refused';
	} else if (error.message?.includes('net::ERR_CERT_')) {
		return 'SSL certificate error';
	} else if (error.message?.includes('net::ERR_INTERNET_DISCONNECTED')) {
		return 'No internet connection';
	} else if (error.message?.includes('net::ERR_BLOCKED_BY_CLIENT')) {
		return 'Blocked by SSRF guard: address not allowed';
	}
	return `Navigation error: ${error.message || 'Unknown error'}`;
}

async function navigateToPage(
	page: pup.Page,
	targetUrl: string,
	options: NavigationOptions = {},
): Promise<{ status: number; error?: string; response?: pup.HTTPResponse }> {
	const { waitUntil = 'domcontentloaded', timeout = 30000 } = options;

	try {
		const response = await page.goto(targetUrl, {
			waitUntil,
			timeout,
		});

		if (!response) {
			return {
				status: STATUS_I_AM_A_TEAPOT,
				error: 'Navigation did not result in a valid HTTP response',
			};
		}

		return { status: response.status(), response };
	} catch (navigationError: any) {
		logger.log('error', 'navigation Error', navigationError);
		const statusCode = getNavigationErrorStatus(navigationError);
		const errorMessage = getNavigationErrorMessage(navigationError);
		return { status: statusCode, error: errorMessage };
	}
}

async function asText(page: pup.Page) {
	const bodyText = await page.evaluate(() => {
		return document.body.innerText;
	});

	return normalizeWhitespace(bodyText);
}

async function asHTML(page: pup.Page) {
	return await page.evaluate(() => {
		return document.body.innerHTML;
	});
}

async function asScreenshot(page: pup.Page) {
	return await page.screenshot();
}

class HeapLimitExceeded extends Error {
	constructor(heapMB: number, maxHeapMB: number) {
		super(`Page JS heap ${heapMB.toFixed(1)}MB exceeds limit ${maxHeapMB}MB`);
	}
}

const HEAP_CHECK_INTERVAL_MS = envInt(
	'GVM_WEBDRIVER_HEAP_CHECK_INTERVAL_MS',
	200,
);

async function withHeapMonitor<T>(
	page: pup.Page,
	maxHeapMB: number,
	fn: () => Promise<T>,
): Promise<T> {
	let stopped = false;
	let stopResolve: () => void;
	const stopPromise = new Promise<void>((r) => {
		stopResolve = r;
	});
	const monitor = (async () => {
		while (!stopped) {
			await new Promise((r) => setTimeout(r, HEAP_CHECK_INTERVAL_MS));
			if (stopped) break;
			let totalMB: number;
			try {
				totalMB = await page.evaluate(
					() => (performance as any).memory?.totalJSHeapSize / 1024 / 1024,
				);
			} catch {
				await stopPromise;
				return;
			}
			logger.log('debug', 'heap monitor check', {
				heapMB: totalMB.toFixed(1),
			});
			if (totalMB > maxHeapMB) {
				throw new HeapLimitExceeded(totalMB, maxHeapMB);
			}
		}
	})();

	try {
		const result = await Promise.race([
			fn(),
			monitor.then(() => undefined as never),
		]);
		let totalMB: number;
		try {
			totalMB = await page.evaluate(
				() => (performance as any).memory?.totalJSHeapSize / 1024 / 1024,
			);
		} catch {
			return result;
		}
		if (totalMB > maxHeapMB) {
			throw new HeapLimitExceeded(totalMB, maxHeapMB);
		}
		return result;
	} finally {
		stopped = true;
		stopResolve!();
		await monitor.catch(() => {});
	}
}

function statusIsGood(status: number): boolean {
	return (status >= 200 && status < 300) || status === 304;
}

async function renderPage(
	targetUrl: string,
	mode: 'text' | 'html' | 'screenshot',
	options: RenderOptions = {},
): Promise<{ status: number; body: any }> {
	const browserManager = await chromeBrowser.INSTANCE;
	const browserInstance = browserManager.getBrowser();
	try {
		return renderPageWithBrowser(
			browserInstance.get(),
			targetUrl,
			mode,
			options,
		);
	} finally {
		browserInstance.close();
	}
}

async function renderPageWithBrowser(
	browserInstance: pup.Browser,
	targetUrl: string,
	mode: 'text' | 'html' | 'screenshot',
	options: RenderOptions = {},
): Promise<{ status: number; body: any }> {
	const {
		loadTimeout = 30000,
		waitAfterLoaded = 0,
		waitUntil = 'domcontentloaded',
		maxPageHeapMB = DEFAULT_MAX_PAGE_HEAP_MB,
	} = options;

	const page = await browserInstance.newPage();

	try {
		await ssrf.installSsrfGuard(page);
		page.setViewport({ width: 1920 / 2, height: 1080 / 2 });

		return await withHeapMonitor(page, maxPageHeapMB, async () => {
			const navigationResult = await navigateToPage(page, targetUrl, {
				waitUntil,
				timeout: loadTimeout,
			});

			if (navigationResult.error) {
				return {
					status: navigationResult.status,
					body: navigationResult.error,
				};
			}

			const statusCode = navigationResult.status;

			if (statusIsGood(statusCode) && waitAfterLoaded > 0) {
				let waitMs = Math.floor(waitAfterLoaded * 1000);
				if (waitMs > MAX_WAIT_AFTER_LOADED_MS) {
					logger.log('warn', 'waitAfterLoaded clamped to maximum', {
						url: targetUrl,
						requested: formatDurationMs(waitMs),
						max: formatDurationMs(MAX_WAIT_AFTER_LOADED_MS),
					});
					waitMs = MAX_WAIT_AFTER_LOADED_MS;
				}
				await new Promise((resolve) => setTimeout(resolve, waitMs));
			}

			let data;
			switch (mode) {
				case 'text':
					data = await asText(page);
					break;
				case 'html':
					data = await asHTML(page);
					break;
				case 'screenshot':
					data = await asScreenshot(page);
					break;
				default:
					data = 'Invalid mode';
			}

			return { status: statusCode, body: data };
		});
	} catch (e) {
		if (e instanceof HeapLimitExceeded) {
			logger.log('warn', 'page heap limit exceeded', {
				url: targetUrl,
				error: e.message,
			});
			return { status: 507, body: e.message };
		}
		throw e;
	} finally {
		await page.close();
	}
}

async function handleRenderRequest(
	parsedUrl: URL,
	req: http.IncomingMessage,
	res: http.ServerResponse,
) {
	const query = parsedUrl.searchParams;

	try {
		const targetUrl = query.get('url') ?? '';
		const mode = query.get('mode') as 'text' | 'html' | 'screenshot';

		if (!targetUrl) {
			res.writeHead(400, { 'Content-Type': 'application/json' });
			res.end(JSON.stringify({ error: 'Missing url parameter' }));
			return;
		}

		if (!['text', 'html', 'screenshot'].includes(mode)) {
			res.writeHead(400, { 'Content-Type': 'application/json' });
			res.end(
				JSON.stringify({
					error: 'Invalid mode. Must be text, html, or screenshot',
				}),
			);
			return;
		}

		const options: RenderOptions = {
			waitAfterLoaded: parseFloat(query.get('waitAfterLoaded') || '0'),
			loadTimeout: parseInt(query.get('loadTimeout') || '30000'),
			waitUntil:
				(query.get('waitUntil') as pup.PuppeteerLifeCycleEvent) ||
				'domcontentloaded',
			...(query.get('maxPageHeapMB')
				? { maxPageHeapMB: parseInt(query.get('maxPageHeapMB')!) }
				: {}),
		};

		const result = await renderPage(targetUrl, mode, options);

		if (statusIsGood(result.status)) {
			updateLastSuccessfulRenderTime();
		}

		res.setHeader('Resulting-Status', result.status.toString());

		if (mode === 'screenshot') {
			res.writeHead(200, { 'Content-Type': 'image/png' });
		} else {
			res.writeHead(200, { 'Content-Type': 'application/json' });
		}
		res.end(result.body);
	} catch (error) {
		res.writeHead(500, { 'Content-Type': 'application/json' });
		res.end(
			JSON.stringify({
				error: 'Internal server error',
				message: (error as Error).message,
			}),
		);
	}
}

async function handleHealthcheck(parsedUrl: URL, res: http.ServerResponse) {
	const now = Date.now();
	const sinceLastSuccessMs = now - lastSuccessfulRenderTime;
	if (sinceLastSuccessMs < HEALTHCHECK_CACHE_DURATION_MS) {
		logger.log('debug', 'healthcheck cached ok', {
			lastSuccessAgo: formatDurationMs(sinceLastSuccessMs),
		});
		res.writeHead(200, { 'Content-Type': 'text/plain' });
		res.end('ok');
		return;
	}

	const query = parsedUrl.searchParams;
	const targetUrl = query.get('url') ?? '';
	const VALID_MODES = ['text', 'html', 'screenshot'] as const;
	const rawMode = query.get('mode');
	const mode = VALID_MODES.find((m) => m === rawMode);

	if (!targetUrl || !mode) {
		logger.log('warn', 'healthcheck missing parameters', {
			url: targetUrl,
			mode: rawMode,
		});
		res.writeHead(400, { 'Content-Type': 'text/plain' });
		res.end('missing url or mode query parameters');
		return;
	}

	logger.log('info', 'healthcheck performing render', {
		url: targetUrl,
		mode,
		sinceLast: formatDurationMs(sinceLastSuccessMs),
		cacheDuration: formatDurationMs(HEALTHCHECK_CACHE_DURATION_MS),
	});
	try {
		const result = await renderPage(targetUrl, mode, {});
		if (statusIsGood(result.status)) {
			updateLastSuccessfulRenderTime();
			logger.log('info', 'healthcheck ok', { status: result.status });
			res.writeHead(200, { 'Content-Type': 'text/plain' });
			res.end('ok');
		} else {
			logger.log('warn', 'healthcheck unhealthy', { status: result.status });
			res.writeHead(503, { 'Content-Type': 'text/plain' });
			res.end('unhealthy');
		}
	} catch (error) {
		logger.log('error', 'healthcheck error', {
			error: (error as Error).message,
		});
		res.writeHead(503, { 'Content-Type': 'text/plain' });
		res.end('unhealthy');
	}
}

const server = http.createServer(async (req, res) => {
	const parsedUrl = new URL(req.url || '/', 'http://localhost');
	const pathname = parsedUrl.pathname;

	if (pathname === '/render') {
		await handleRenderRequest(parsedUrl, req, res);
	} else if (pathname === '/healthcheck') {
		await handleHealthcheck(parsedUrl, res);
	} else if (pathname === '/log-level') {
		if (req.method === 'GET') {
			res.writeHead(200, { 'Content-Type': 'text/plain' });
			res.end(logger.MIN_LEVEL);
			return;
		}
		if (req.method !== 'POST') {
			res.writeHead(405, { 'Content-Type': 'text/plain' });
			res.end('method not allowed');
			return;
		}
		const level = parsedUrl.searchParams.get('level');
		if (!level || !logger.VALID_LEVELS.includes(level as logger.LogLevel)) {
			res.writeHead(400, { 'Content-Type': 'text/plain' });
			res.end(
				`invalid level, must be one of: ${logger.VALID_LEVELS.join(', ')}`,
			);
			return;
		}
		const prev = logger.MIN_LEVEL;
		logger.setMinLevel(level as logger.LogLevel);
		logger.log('info', 'log level changed', { from: prev, to: level });
		res.writeHead(200, { 'Content-Type': 'text/plain' });
		res.end(level);
	} else {
		res.writeHead(404, { 'Content-Type': 'text/plain' });
		res.end(
			'Puppeteer webdriver server. Use /render endpoint with url, mode, waitAfterLoaded, and waitUntil parameters.',
		);
	}
});

const port = parseInt(options.port);

import * as proc from 'process';
import { log } from 'console';

server.listen(port, () => {
	logger.log('info', 'server started', {
		port,
		url: `http://localhost:${port}/`,
		pid: proc.pid,
	});
});
