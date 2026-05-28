import * as util from 'util';

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export let MIN_LEVEL: LogLevel = 'info';

export const VALID_LEVELS: readonly LogLevel[] = [
	'debug',
	'info',
	'warn',
	'error',
];

const LEVELS: { [key in LogLevel]: number } = {
	debug: 10,
	info: 20,
	warn: 30,
	error: 40,
};

export function setMinLevel(level: LogLevel): void {
	MIN_LEVEL = level;
}

export function log(
	level: LogLevel,
	message: string,
	data?: { [key: string]: any },
): void {
	if (LEVELS[level] < LEVELS[MIN_LEVEL]) {
		return;
	}

	const obj = {
		level,
		message,
		...data,
	};

	console.log(util.inspect(obj, { depth: Infinity, breakLength: Infinity }));
}
