import puppeteer, * as pup from 'puppeteer-core';
import * as logger from './logging.js'

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
		logger.log('info', 'created browser', {id: this.id});
	}

	async close(): Promise<void> {
		this.counter--;
		if (this.counter === 0) {
			logger.log('info', 'closing browser instance', {id: this.id});
			await this.browser.close();
		}
	}

	get(): pup.Browser {
		return this.browser;
	}
}

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
			'--disable-gpu'
		],
		executablePath: '/usr/bin/chromium',
	});

	logger.log('info', 'created new raw browser', {'pid': realBrowser.process()?.pid});

	realBrowser.on('disconnected', () => {
		logger.log('info', 'browser disconnected');
	})

	return new BrowserHolder(realBrowser);
}

export class BrowserManager {
	private holder: BrowserHolder;

	private constructor(holder: BrowserHolder) {
		this.holder = holder;
		setInterval(async () => {
			const old = this.holder;
			const newB = await newBrowser();
			this.holder = newB;
			await old.close();
		}, 10 * 60 * 1000) // Rotate every 10 minutes
	}

	getBrowser(): BrowserHandle {
		this.holder.counter++;
		return this.holder;
	}

	static INSTANCE: Promise<BrowserManager> = (async () => {
		const browser = await newBrowser();
		return new BrowserManager(browser);
	})();
}
