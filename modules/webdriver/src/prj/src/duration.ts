import * as logger from './logging.js';

/**
 * Parse a duration string with optional suffix: "500ms", "30s", "5m".
 * Plain numbers are treated as milliseconds.
 * Returns the value in milliseconds, or `defaultMs` if the input is empty/undefined.
 */
export function parseDurationMs<D = number>(
	input: string | undefined,
	defaultMs: D,
): number | D {
	if (input === undefined || input === '') {
		return defaultMs;
	}
	if (input === '0') {
		return 0;
	}
	const trimmed = input.trim();
	const match = trimmed.match(/^(\d+(?:\.\d+)?)\s*(ms|s|m|h)?$/);
	if (!match || !match[1]) {
		throw new Error(`invalid duration: "${input}"`);
	}
	const value = parseFloat(match[1]);
	switch (match[2] ?? 'ms') {
		case 'ms':
			return value;
		case 's':
			return value * 1000;
		case 'm':
			return value * 60 * 1000;
		case 'h':
			return value * 60 * 60 * 1000;
		default:
			throw new Error(`invalid duration suffix: "${match[2]}"`);
	}
}

export function envDurationMs(envName: string, dflt: number | string): number {
	let raw = process.env[envName];
	const value1 = parseDurationMs(raw, dflt);
	const value =
		typeof value1 === 'string' ? parseDurationMs(value1, undefined) : value1;
	if (value === undefined) {
		throw new Error(`invalid duration for env ${envName}: "${raw}"`);
	}
	logger.log('info', 'env', { name: envName, raw: raw ?? null, value, dflt });
	return value;
}

export function formatDurationMs(ms: number): string {
	if (ms < 1000) return `${ms}ms`;
	if (ms < 60 * 1000) return `${(ms / 1000).toFixed(1)}s`;
	if (ms < 60 * 60 * 1000) return `${(ms / 60000).toFixed(1)}m`;
	return `${(ms / 3600000).toFixed(1)}h`;
}

export function envInt(envName: string, defaultValue: number): number {
	const raw = process.env[envName];
	const value = raw !== undefined && raw !== '' ? parseInt(raw, 10) : NaN;
	const result = Number.isNaN(value) ? defaultValue : value;
	logger.log('info', 'env', {
		name: envName,
		raw: raw ?? null,
		value: result,
		default: defaultValue,
	});
	return result;
}
