import * as pup from 'puppeteer-core';

export interface Handle {
	close(): Promise<void>;
	get(): pup.Browser;
}

export interface Manager {
	getBrowser(): Handle;
}
