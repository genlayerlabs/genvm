import puppeteer, * as pup from 'puppeteer-core';
import http from 'http';
import url from 'url';
import { Command } from 'commander';

import * as logger from './logging.js';
import * as browser from './browser.js';

interface NavigationOptions {
	waitUntil?: pup.PuppeteerLifeCycleEvent;
	timeout?: number;
}

interface RenderOptions {
	loadTimeout?: number;
	waitAfterLoaded?: number;
	waitUntil?: pup.PuppeteerLifeCycleEvent;
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

function normalizeWhitespace(contents: string): string {
	return contents
		.split('\n')
		.map(line => line.trim().replace(/\s+/g, ' '))
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
	}
	return `Navigation error: ${error.message || 'Unknown error'}`;
}

async function navigateToPage(page: pup.Page, targetUrl: string, options: NavigationOptions = {}): Promise<{ status: number; error?: string; response?: pup.HTTPResponse }> {
	const { waitUntil = 'domcontentloaded', timeout = 30000 } = options;

	try {
		const response = await page.goto(targetUrl, {
			waitUntil,
			timeout
		});

		if (!response) {
			return { status: STATUS_I_AM_A_TEAPOT, error: 'Navigation did not result in a valid HTTP response' };
		}

		return { status: response.status(), response };
	} catch (navigationError: any) {
		logger.log('error', 'navigation Error', navigationError)
		const statusCode = getNavigationErrorStatus(navigationError);
		const errorMessage = getNavigationErrorMessage(navigationError);
		return { status: statusCode, error: errorMessage };
	}
}

async function asText(page: pup.Page) {
	const bodyText = await page.evaluate(() => {
		return document.body.innerText;
	});

	return normalizeWhitespace(bodyText)
}


async function asHTML(page: pup.Page) {
	return await page.evaluate(() => {
		return document.body.innerHTML;
	})
}

async function asScreenshot(page: pup.Page) {
	return await page.screenshot();
}

function statusIsGood(status: number): boolean {
	return status >= 200 && status < 300 || status === 304;
}

async function renderPage(targetUrl: string, mode: 'text' | 'html' | 'screenshot', options: RenderOptions = {}): Promise<{status: number, body: any}> {
	const browserManager = await browser.BrowserManager.INSTANCE;
	const browserInstance = browserManager.getBrowser();
	try {
		return renderPageWithBrowser(browserInstance.get(), targetUrl, mode, options);
	} finally {
		browserInstance.close();
	}
}

async function renderPageWithBrowser(browserInstance: pup.Browser, targetUrl: string, mode: 'text' | 'html' | 'screenshot', options: RenderOptions = {}): Promise<{status: number, body: any}> {
	const { loadTimeout = 30000, waitAfterLoaded = 0, waitUntil = 'domcontentloaded' } = options;

	const page = await browserInstance.newPage();

	try {
		page.setViewport({ width: 1920/2, height: 1080/2 });

		const navigationResult = await navigateToPage(page, targetUrl, { waitUntil, timeout: loadTimeout });

		// If navigation failed, return the error immediately
		if (navigationResult.error) {
			return { status: navigationResult.status, body: navigationResult.error };
		}

		const statusCode = navigationResult.status;

		// Wait after loaded if status is 200
		if (statusIsGood(statusCode) && waitAfterLoaded > 0) {
			await new Promise(resolve => setTimeout(resolve, Math.floor(waitAfterLoaded * 1000)));
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
	} finally {
		await page.close();
	}
}

async function handleRenderRequest(parsedUrl: url.UrlWithParsedQuery, req: http.IncomingMessage, res: http.ServerResponse) {
	const query = parsedUrl.query;

	try {
		const targetUrl = query.url as string;
		const mode = query.mode as 'text' | 'html' | 'screenshot';

		if (!targetUrl) {
			res.writeHead(400, {'Content-Type': 'application/json'});
			res.end(JSON.stringify({ error: 'Missing url parameter' }));
			return;
		}

		if (!['text', 'html', 'screenshot'].includes(mode)) {
			res.writeHead(400, {'Content-Type': 'application/json'});
			res.end(JSON.stringify({ error: 'Invalid mode. Must be text, html, or screenshot' }));
			return;
		}

		const options: RenderOptions = {
			waitAfterLoaded: parseFloat(query.waitAfterLoaded as string || '0'),
			loadTimeout: parseInt(query.loadTimeout as string || '30000'),
			waitUntil: (query.waitUntil as pup.PuppeteerLifeCycleEvent) || 'domcontentloaded'
		};

		const result = await renderPage(targetUrl, mode, options);

		res.setHeader('Resulting-Status', result.status.toString());

		if (mode === 'screenshot') {
			res.writeHead(200, {'Content-Type': 'image/png'});
		} else {
			res.writeHead(200, {'Content-Type': 'application/json'});
		}
		res.end(result.body);
	} catch (error) {
		res.writeHead(500, {'Content-Type': 'application/json'});
		res.end(JSON.stringify({ error: 'Internal server error', message: (error as Error).message }));
	}
}

const server = http.createServer(async (req, res) => {
	const parsedUrl = url.parse(req.url || '', true);
	const pathname = parsedUrl.pathname;

	if (pathname === '/render') {
		await handleRenderRequest(parsedUrl, req, res);
	} else {
		res.writeHead(404, {'Content-Type': 'text/plain'});
		res.end('Puppeteer webdriver server. Use /render endpoint with url, mode, waitAfterLoaded, and waitUntil parameters.');
	}
});

const port = parseInt(options.port);

import * as proc from 'process';

server.listen(port, () => {
	logger.log('info', 'server started', {port, 'url': `http://localhost:${port}/`, pid: proc.pid});
});
