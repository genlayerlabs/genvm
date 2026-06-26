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

export function envStr(envName: string, defaultValue: string): string {
	const raw = process.env[envName];
	const value = raw !== undefined && raw !== '' ? raw : defaultValue;
	logger.log('info', 'env', {
		name: envName,
		raw: raw ?? null,
		value,
		default: defaultValue,
	});
	return value;
}

const ENV_BOOL_TRUE = ['1', 'true', 'yes', 'on'];
const ENV_BOOL_FALSE = ['0', 'false', 'no', 'off'];

export function envBool(envName: string, defaultValue: boolean): boolean {
	const raw = process.env[envName];
	let value = defaultValue;
	if (raw !== undefined && raw !== '') {
		const norm = raw.trim().toLowerCase();
		if (ENV_BOOL_TRUE.includes(norm)) {
			value = true;
		} else if (ENV_BOOL_FALSE.includes(norm)) {
			value = false;
		} else {
			throw new Error(`invalid boolean for env ${envName}: "${raw}"`);
		}
	}
	logger.log('info', 'env', {
		name: envName,
		raw: raw ?? null,
		value,
		default: defaultValue,
	});
	return value;
}

/**
 * Parse a list of strings from an env var. A value starting with `[` is parsed
 * as a JSON array (use this when arguments contain spaces); otherwise the value
 * is split on whitespace.
 */
export function envStrList(envName: string, defaultValue: string[]): string[] {
	const raw = process.env[envName];
	let value = defaultValue;
	const trimmed = raw?.trim() ?? '';
	if (trimmed !== '') {
		if (trimmed.startsWith('[')) {
			const parsed: unknown = JSON.parse(trimmed);
			if (!Array.isArray(parsed) || parsed.some((x) => typeof x !== 'string')) {
				throw new Error(`invalid string list for env ${envName}: "${raw}"`);
			}
			value = parsed as string[];
		} else {
			value = trimmed.split(/\s+/);
		}
	}
	logger.log('info', 'env', {
		name: envName,
		raw: raw ?? null,
		value,
		default: defaultValue,
	});
	return value;
}
