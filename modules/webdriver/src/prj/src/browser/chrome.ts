import puppeteer, * as pup from 'puppeteer-core';
import * as logger from '../logging.js';
import * as util from 'util';
import * as child_process from 'child_process';
import {
	envBool,
	envDurationMs,
	envInt,
	envStr,
	envStrList,
	formatDurationMs,
} from '../duration.js';
import type * as browser from './index.js';

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

const ROTATION_CHECK_INTERVAL_MS = envDurationMs(
	'GVM_WEBDRIVER_ROTATION_CHECK_INTERVAL',
	'10m',
);
const ROTATION_ABSOLUTE_THRESHOLD_MB = envInt(
	'GVM_WEBDRIVER_ROTATION_ABSOLUTE_MB',
	2048,
);
const ROTATION_IDLE_DELTA_MB = envInt(
	'GVM_WEBDRIVER_ROTATION_IDLE_DELTA_MB',
	256,
);
const ROTATION_AT_LEAST_MS = envDurationMs(
	'GVM_WEBDRIVER_ROTATION_AT_LEAST',
	0,
);
const RENDERER_MAX_OLD_SPACE_MB = envInt(
	'GVM_WEBDRIVER_RENDERER_MAX_OLD_SPACE_MB',
	1024,
);
const RENDERER_PROCESS_LIMIT = envInt(
	'GVM_WEBDRIVER_RENDERER_PROCESS_LIMIT',
	4,
);

const CHROME_EXECUTABLE = envStr(
	'GVM_WEBDRIVER_CHROME_EXECUTABLE',
	'/usr/bin/chromium',
);
const CHROME_HEADLESS = envBool('GVM_WEBDRIVER_CHROME_HEADLESS', true);
// Extra flags appended to the defaults below. See `envStrList` for the format.
const CHROME_EXTRA_ARGS = envStrList('GVM_WEBDRIVER_CHROME_ARGS', []);

async function newBrowser(): Promise<BrowserHolder> {
	const args = [
		'--no-sandbox',
		'--disable-dev-shm-usage',
		'--disable-accelerated-2d-canvas',
		'--no-first-run',
		'--no-zygote',
		'--disable-gpu',
		'--enable-precise-memory-info',
		`--js-flags=--max-old-space-size=${RENDERER_MAX_OLD_SPACE_MB}`,
		`--renderer-process-limit=${RENDERER_PROCESS_LIMIT}`,
		...CHROME_EXTRA_ARGS,
	];
	const realBrowser = await puppeteer.launch({
		headless: CHROME_HEADLESS,
		args,
		executablePath: CHROME_EXECUTABLE,
	});

	logger.log('info', 'created new raw browser', {
		pid: realBrowser.process()?.pid,
	});

	realBrowser.on('disconnected', () => {
		logger.log('info', 'browser disconnected');
	});

	let pid = realBrowser.process()?.pid;
	if (!initialMemoryMB && pid) {
		const selfMB = await getTotalRssMB(pid);
		if (!initialMemoryMB) {
			initialMemoryMB = selfMB;
		}
	}

	return new BrowserHolder(realBrowser);
}

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

class ChromeBrowserManager implements browser.Manager {
	private holder: BrowserHolder;
	private used: boolean;
	private lastMemoryMB: number | null = null;
	private lastRotationTime: number = Date.now();

	private constructor(holder: BrowserHolder) {
		this.holder = holder;
		this.used = false;
		this.scheduleRotationCheck();
	}

	/**
	 * Arm the next rotation check. Uses a self-rescheduling timeout instead of
	 * setInterval so the next check is only scheduled once the current one has
	 * fully settled (in `finally`). This structurally prevents overlapping
	 * rotations from interleaving holder swaps / closes — which could otherwise
	 * orphan a freshly launched browser (process leak) or close one still
	 * serving a render — and avoids interval backlog under sustained load.
	 */
	private scheduleRotationCheck(): void {
		setTimeout(() => {
			void this.rotationCheck()
				.catch((error) => {
					logger.log('error', 'browser rotation check failed', { error });
				})
				.finally(() => {
					this.scheduleRotationCheck();
				});
		}, ROTATION_CHECK_INTERVAL_MS);
	}

	private async rotationCheck(): Promise<void> {
		const used = this.used;
		const lastMemoryMB = this.lastMemoryMB;
		const old = this.holder;
		const proc = old.browser.process();
		const pid = proc?.pid;
		let usedRAMMB = pid ? await getTotalRssMB(pid) : await getSelfRssMB();
		this.lastMemoryMB = usedRAMMB;

		let memoryDeltaMB = lastMemoryMB !== null ? usedRAMMB - lastMemoryMB : 0;

		const reasons: string[] = [];

		if (
			initialMemoryMB &&
			usedRAMMB > initialMemoryMB + ROTATION_ABSOLUTE_THRESHOLD_MB
		) {
			reasons.push(
				`absolute memory ${usedRAMMB.toFixed(0)}MB exceeds initial ${initialMemoryMB.toFixed(0)}MB + ${ROTATION_ABSOLUTE_THRESHOLD_MB}MB`,
			);
		}

		if (!used && memoryDeltaMB > ROTATION_IDLE_DELTA_MB) {
			reasons.push(
				`idle memory grew by ${memoryDeltaMB.toFixed(0)}MB (threshold ${ROTATION_IDLE_DELTA_MB}MB)`,
			);
		}

		if (
			ROTATION_AT_LEAST_MS > 0 &&
			Date.now() - this.lastRotationTime >= ROTATION_AT_LEAST_MS
		) {
			reasons.push(
				`age ${formatDurationMs(Date.now() - this.lastRotationTime)} exceeds max ${formatDurationMs(ROTATION_AT_LEAST_MS)}`,
			);
		}

		logger.log('info', 'browser rotation check', {
			shouldRotate: reasons.length > 0,
			reasons,
			used,
			usedRAMMB,
			memoryDeltaMB,
			initialMemoryMB,
			lastRotationAgo: formatDurationMs(Date.now() - this.lastRotationTime),
		});

		if (reasons.length === 0) {
			return;
		}

		const newB = await newBrowser();
		this.holder = newB;
		this.lastMemoryMB = null;
		this.used = false;
		this.lastRotationTime = Date.now();
		await old.close();
	}

	getBrowser(): browser.Handle {
		this.holder.counter++;
		this.used = true;
		return this.holder;
	}

	static create(): Promise<ChromeBrowserManager> {
		return (async () => {
			const browser = await newBrowser();
			return new ChromeBrowserManager(browser);
		})();
	}
}

export const INSTANCE: Promise<browser.Manager> =
	ChromeBrowserManager.create().catch((error) => {
		logger.log('error', 'failed to create initial browser', { error });
		process.exit(1);
	});
