import puppeteer, * as pup from 'puppeteer-core';
import * as logger from './logging.js';
import * as util from 'util';

export interface BrowserHandle {
	close(): Promise<void>;
	get(): pup.Browser;
}

class BrowserHolder {
	browser: pup.Browser;
	counter: number;
	id: number;

	static nextId = 1;

	constructor(browser: pup.Browser) {
		this.browser = browser;
		this.counter = 1;
		this.id = BrowserHolder.nextId++;
		logger.log('info', 'created browser', { id: this.id });
	}

	async close(): Promise<void> {
		this.counter--;
		if (this.counter === 0) {
			logger.log('info', 'closing browser instance', { id: this.id });
			await this.browser.close();
		}
	}

	get(): pup.Browser {
		return this.browser;
	}
}

let initialMemoryMB: number | undefined;

async function newBrowser(): Promise<BrowserHolder> {
	const realBrowser = await puppeteer.launch({
		headless: true,
		args: [
			'--no-sandbox',
			'--disable-dev-shm-usage',
			'--disable-accelerated-2d-canvas',
			'--no-first-run',
			'--single-process',
			'--no-zygote',
			'--disable-gpu',
		],
		executablePath: '/usr/bin/chromium',
	});

	logger.log('info', 'created new raw browser', {
		pid: realBrowser.process()?.pid,
	});

	realBrowser.on('disconnected', () => {
		logger.log('info', 'browser disconnected');
	});

	let pid = realBrowser.process()?.pid;
	if (!initialMemoryMB && pid) {
		const selfMB = await getSelfRssMB();
		if (!initialMemoryMB) {
			initialMemoryMB = selfMB;
		}
	}

	return new BrowserHolder(realBrowser);
}

import * as child_process from 'child_process';

async function getRssMBByPid(pid: number): Promise<number> {
	try {
		const output = await util.promisify(child_process.exec)(
			`ps -o rss= -p ${pid}`,
		);
		return parseInt(output.stdout.trim(), 10) / 1024;
	} catch (error) {
		logger.log('error', 'Failed to get RSS memory usage', { pid, error });
		return 0;
	}
}

async function getSelfRssMB(): Promise<number> {
	try {
		return process.memoryUsage().rss / 1024 / 1024;
	} catch (error) {
		logger.log('error', 'Failed to get self RSS memory usage', { error });
		return 0;
	}
}

async function getTotalRssMB(pid: number): Promise<number> {
	return (await getSelfRssMB()) + (await getRssMBByPid(pid));
}

export class BrowserManager {
	private holder: BrowserHolder;
	private used: boolean;
	private lastMemoryMB: number | null = null;

	private constructor(holder: BrowserHolder) {
		this.holder = holder;
		this.used = false;
		setInterval(
			async () => {
				const used = this.used;
				const lastMemoryMB = this.lastMemoryMB;
				const old = this.holder;
				const proc = old.browser.process();
				const pid = proc?.pid;
				let usedRAMMB = pid ? await getTotalRssMB(pid) : await getSelfRssMB();
				this.lastMemoryMB = usedRAMMB;

				let memoryDeltaMB =
					lastMemoryMB !== null ? usedRAMMB - lastMemoryMB : 0;

				let shouldRotate = false;

				if (initialMemoryMB && usedRAMMB > initialMemoryMB + 2048) {
					shouldRotate = true;
				}

				if (!used && memoryDeltaMB > 256) {
					shouldRotate = true;
				}

				logger.log('info', 'browser rotation check', {
					shouldRotate,
					used,
					usedRAMMB,
					memoryDeltaMB,
					initialMemoryMB,
				});

				if (!shouldRotate) {
					return;
				}

				const newB = await newBrowser();
				this.holder = newB;
				this.lastMemoryMB = null;
				await old.close();
			},
			10 * 60 * 1000,
		); // Rotate every 10 minutes
	}

	getBrowser(): BrowserHandle {
		this.holder.counter++;
		this.used = true;
		return this.holder;
	}

	static INSTANCE: Promise<BrowserManager> = (async () => {
		const browser = await newBrowser();
		return new BrowserManager(browser);
	})();
}
